use super::file;
use super::{PollData, PollType};
use crate::time;
use crate::enums::TradeOfferState;
use crate::types::TradeOfferId;
use crate::response::TradeOffer;
use crate::api::SteamTradeOfferAPI;
use crate::error::Error;
use std::path::PathBuf;
use std::collections::HashMap;
use chrono::Duration;
use steamid_ng::SteamID;

pub type Poll = Vec<(TradeOffer, Option<TradeOfferState>)>;
pub type PollResult = Result<Poll, Error>;

const OFFERS_SINCE_BUFFER_SECONDS: i64 = 60 * 30;
const OFFERS_SINCE_ALL_TIMESTAMP: i64 = 1;
const STATE_MAP_SIZE_LIMIT: usize = 2500;
const STATE_MAP_SPLIT_AT: usize = 2000;

pub struct Poller {
    pub steamid: SteamID,
    pub api: SteamTradeOfferAPI,
    pub data_directory: PathBuf,
    pub cancel_duration: Option<Duration>,
    pub poll_full_update_duration: Duration,
    pub poll_data: PollData,
}

impl Poller {
    /// Performs a poll for changes to offers. Provides a parameter to determine what type of poll to perform.
    pub async fn do_poll(
        &mut self,
        poll_type: PollType,
    ) -> PollResult {
        let now = time::get_server_time_now();
        let mut offers_since = self.poll_data.offers_since
            // Steam can be dumb and backdate a modified offer. We need to handle this by adding a buffer.
            .map(|date| date.timestamp() - OFFERS_SINCE_BUFFER_SECONDS)
            .unwrap_or(OFFERS_SINCE_ALL_TIMESTAMP);
        let mut active_only = true;
        let mut full_update = {
            poll_type.is_full_update() || 
            // The date of the last full poll is outdated.
            self.poll_data.last_full_poll_is_stale(&self.poll_full_update_duration)
        };
        
        if poll_type == PollType::NewOffers {
            // a very high date
            offers_since = u32::MAX as i64;
            full_update = false;
        } else if let PollType::OffersSince(date) = poll_type {
            offers_since = date.timestamp();
            active_only = false;
            full_update = false;
        } else if full_update {
            offers_since = OFFERS_SINCE_ALL_TIMESTAMP;
            active_only = false;
        }
        
        let (mut offers, descriptions) = self.api.get_raw_trade_offers(
            active_only,
            false,
            true,
            true,
            poll_type.is_active_only(),
            Some(time::timestamp_to_server_time(offers_since)),
        ).await?;
        
        if !poll_type.is_active_only() {
            self.poll_data.set_last_poll(now);
        }
        
        if full_update {
            self.poll_data.set_last_poll_full_update(now);
        }
        
        // Vec of offers that were cancelled.
        let cancelled_offers = if let Some(cancel_duration) = self.cancel_duration {
            let cancel_time = chrono::Utc::now() - cancel_duration;
            // Cancels all offers older than cancel_time.
            let cancel_futures = offers
                .iter_mut()
                .filter(|offer| {
                    let is_active_state = {
                        offer.trade_offer_state == TradeOfferState::Active ||
                        offer.trade_offer_state == TradeOfferState::CreatedNeedsConfirmation
                    };
                    
                    is_active_state &&
                    offer.is_our_offer &&
                    offer.time_created < cancel_time
                })
                .map(|offer| self.api.cancel_offer(offer.tradeofferid))
                .collect::<Vec<_>>();
            
            futures::future::join_all(cancel_futures).await
                .into_iter()
                .filter_map(|offer| offer.ok())
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        // For reducing file writes, keep track of whether the state of poll data has changed.
        let mut prev_states_map: HashMap<TradeOfferId, TradeOfferState> = HashMap::new();
        let mut poll: Vec<_> = Vec::new();
        let mut offers_since = self.poll_data.offers_since
            .unwrap_or_else(|| time::timestamp_to_server_time(offers_since));
        
        for mut offer in offers {
            // This offer was successfully cancelled above...
            // We need to update its state here.
            if cancelled_offers.contains(&offer.tradeofferid) {
                offer.trade_offer_state = TradeOfferState::Canceled;
            }
            
            // Just don't do anything with this offer.
            if offer.is_glitched() {
                continue;
            }
            
            // Update the offers_since to the most recent trade offer.
            if offer.time_updated > offers_since {
                offers_since = offer.time_updated;
            }
            
            match self.poll_data.state_map.get(&offer.tradeofferid) {
                // State has changed.
                Some(
                    poll_trade_offer_state
                ) if *poll_trade_offer_state != offer.trade_offer_state => {
                    prev_states_map.insert(offer.tradeofferid, *poll_trade_offer_state);
                    poll.push(offer);
                },
                // Nothing has changed...
                Some(_) => {},
                // This is a new offer
                None => poll.push(offer),
            }
        }
        
        if !poll_type.is_active_only() {
            self.poll_data.set_offers_since(offers_since);
        }
        
        // Eventually the state map gets very large. This needs to be trimmed so it does not 
        // expand infintely.
        //
        // This isn't perfect and I may change this later on.
        if self.poll_data.state_map.len() > STATE_MAP_SIZE_LIMIT {
            // Using a higher number than is removed so this process needs to run less frequently.
            let mut tradeofferids = self.poll_data.state_map
                .keys()
                .cloned()
                .collect::<Vec<_>>();
            
            // High to low.
            tradeofferids.sort_by(|a, b| b.cmp(a));
            
            let (
                _tradeofferids,
                tradeofferids_to_remove,
            ) = tradeofferids.split_at(STATE_MAP_SPLIT_AT);
            
            self.poll_data.clear_offers(tradeofferids_to_remove);
        }
        
        // Maps raw offers to offers with classinfo descriptions.
        let offers = if let Some(descriptions) = descriptions {
            self.api.map_raw_trade_offers_with_descriptions(poll, descriptions)
        } else {
            self.api.map_raw_trade_offers(poll).await?
        };
        let poll = if offers.is_empty() {
            // map_raw_trade_offers may have excluded some offers - the state of the poll data
            // is not updated until all descriptions are loaded for the offer
            Vec::new()
        } else {
            self.poll_data.changed = true;
            offers
                .into_iter()
                // Combines changed state maps.
                .map(|offer| {
                    let prev_state = prev_states_map.remove(&offer.tradeofferid);
                    
                    // insert new state into map
                    self.poll_data.state_map.insert(offer.tradeofferid, offer.trade_offer_state);
                    
                    (offer, prev_state)
                })
                .collect::<Vec<_>>()
        };
        
        // Only save if changes were detected.
        if self.poll_data.changed {
            self.poll_data.changed = false;
            // It's really not a problem to await on this.
            // Saving the file takes under a millisecond.
            let _ = file::save_poll_data(
                self.steamid,
                &serde_json::to_string(&self.poll_data)?,
                &self.data_directory,
            ).await;
        }
        
        Ok(poll)
    }
}