#[macro_use]
extern crate dotenv_codegen;

use dotenv::dotenv;
use steam_tradeoffers::{
    TradeOfferManager,
    TradeOfferState,
    response as offers_response,
    request as offers_request,
};
use std::{
    fs::File,
    io::Read,
    time,
    sync::Arc,
    collections::HashMap,
};
use steamid_ng::SteamID;
use lazy_regex::regex_captures;
use async_std::task::sleep;

fn get_cookies(filepath: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let hostname = "steamcommunity.com";
    let mut file = File::open(filepath).unwrap();
    let mut data = String::new();
    
    file.read_to_string(&mut data).unwrap();
    
    let json: HashMap<String, Vec<String>> = serde_json::from_str(&data).expect("JSON was not well-formatted");
    let values = json.get(hostname).expect("No cookies for hostname");
    
    Ok(values.to_owned())
}

#[allow(dead_code)]
fn is_key(item: &offers_response::Asset) -> bool {
    item.appid == 440 && item.classinfo.market_hash_name == "Mann Co. Supply Crate Key"
}

#[allow(dead_code)]
fn metal_value(item: &offers_response::Asset) -> Option<u32> {
    match item.appid {
        440 => {
            match &*item.classinfo.market_hash_name {
                "Refined Metal" => Some(18),
                "Reclaimed Metal" => Some(6),
                "Scrap Metal" => Some(2),
                _ => None,
            }
        },
        _ => None,
    }
}

#[allow(unused_variables)]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    
    let steamid = SteamID::from(76561198130682435);
    let steam_api_key: String = dotenv!("STEAM_API_KEY").to_string();
    let cookies_json_path: String = dotenv!("COOKIES_JSON_PATH").to_string();
    let identity_secret: String = dotenv!("IDENTITY_SECRET").to_string();
    let manager = Arc::new(TradeOfferManager::new(&steamid, &steam_api_key, Some(identity_secret)));
    let cookies = get_cookies(&cookies_json_path)?;
    
    for cookie_str in &cookies {
        if let Some((_, sessionid)) = regex_captures!(r#"sessionid=([A-z0-9]+)"#, cookie_str) {
            let _ = manager.set_session(sessionid, &cookies);
            break;
        }
    }
    
    // match api.send_offer(&offers_request::CreateTradeOffer {
    //     id: None,
    //     items_to_receive: Vec::new(),
    //     items_to_give: vec![
    //         Item {
    //             appid: 440,
    //             contextid: 2,
    //             amount: 1,
    //             assetid: 10863796759,
    //         }
    //     ],
    //     message: Some("hello from rust".to_string()),
    //     partner: steamid,
    //     token: None,
    // }).await {
    //     Ok(res) => {
    //         println!("{:?}", res);
    //     },
    //     Err(err) => println!("{}", err),
    // }
    // thread::sleep(time::Duration::from_secs(10));
    
    // match api.get_inventory_old(&steamid, 440, 2, true).await {
    //     Ok(items) => {
    //         // println!("{}", items.capacity() * std::mem::size_of::<offers_response::Asset>());
    //         // println!("{}", std::mem::size_of::<offers_response::ClassInfo>());
    //         println!("{:?}", items);
    //         if let Some(item) = items.iter().find(|item| is_key(&*item.classinfo)) {
    //             // match api.send_offer(&offers_request::CreateTradeOffer {
    //             //     id: None,
    //             //     items_to_receive: vec![
    //             //         offers_request::CreateTradeOfferItem {
    //             //             appid: 440,
    //             //             contextid: 2,
    //             //             amount: 1,
    //             //             assetid: item.assetid,
    //             //         }
    //             //     ],
    //             //     items_to_give: Vec::new(),
    //             //     message: Some("give me that key".to_string()),
    //             //     partner: steamid,
    //             //     token: None,
    //             // }).await {
    //             //     Ok(res) => println!("{:?}", res),
    //             //     Err(err) => println!("{}", err),
    //             // }
    //         } else {
    //             println!("Can't find that :(");
    //         }
    //     },
    //     Err(err) => println!("{}", err),
    // }
    
    // let thread_manager = Arc::clone(&manager);
    // let handle = tokio::spawn(async move {
    //     loop {
    //         match thread_manager.do_poll(false).await {
    //             Ok(poll) => {
    //                 println!("{:?}", poll);
    //                 for offer in &poll.new {
    //                     match offer.trade_offer_state {
    //                         TradeOfferState::Active => {
    //                             println!("New offer: {}", offer);
    //                         },
    //                         _ => {
    //                             // ignore it...
    //                         }
    //                     }
    //                 }
    //             },
    //             Err(err) => println!("{}", err),
    //         }
            
    //         sleep(time::Duration::from_secs(30)).await;
    //     }
    // });
    
    match manager.get_trade_confirmations().await {
        Ok(offers) => {
            println!("{:?}", offers);
        },
        Err(err) => println!("{}", err),
    }
    
    // handle.await?;
    
    // match api.get_asset_classinfos(&vec![(440, 101785959, 11040578)]).await {
    //     Ok(response) => {
    //         println!("{:?}", response);
    //     },
    //     Err(err) => println!("{}", err),
    // }
    
    Ok(())
}
