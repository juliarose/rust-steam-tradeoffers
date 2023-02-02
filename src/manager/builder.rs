use super::{TradeOfferManager, USER_AGENT_STRING};
use crate::ClassInfoCache;
use std::{path::PathBuf, sync::{Mutex, Arc}};
use reqwest::cookie::Jar;
use reqwest_middleware::ClientWithMiddleware;

/// Builder for constructing a trade offer manager.
pub struct TradeOfferManagerBuilder {
    /// Your account's API key from <https://steamcommunity.com/dev/apikey>.
    pub api_key: String,
    /// The identity secret for the account (optional). Required for mobile confirmations.
    pub identity_secret: Option<String>,
    /// The language for API responses.
    pub language: String,
    /// The [`ClassInfoCache`] to use for this manager. Useful if instantiating multiple managers 
    /// to share state.
    pub classinfo_cache: Arc<Mutex<ClassInfoCache>>,
    /// The location to save data to.
    pub data_directory: PathBuf,
    /// Request cookies.
    pub cookies: Option<Arc<Jar>>,
    /// Client to use for requests. Remember to also include the cookies connected to this client.
    pub client: Option<ClientWithMiddleware>,
    /// User agent for requests.
    pub user_agent: &'static str,
    /// How many seconds your computer is behind Steam's servers. Used in mobile confirmations.
    pub time_offset: i64,
}

impl TradeOfferManagerBuilder {
    /// Creates a new [`TradeOfferManagerBuilder`]. The `data_directory` is the directory used to 
    /// store poll data and classinfo data.
    pub fn new(
        api_key: String,
        data_directory: PathBuf,
    ) -> Self {
        Self {
            api_key,
            identity_secret: None,
            language: String::from("english"),
            classinfo_cache: Arc::new(Mutex::new(ClassInfoCache::default())),
            data_directory,
            cookies: None,
            client: None,
            user_agent: USER_AGENT_STRING,
            time_offset: 0,
        }
    }
    
    /// The identity secret for the account (optional). Required for mobile confirmations.
    pub fn identity_secret(mut self, identity_secret: String) -> Self {
        self.identity_secret = Some(identity_secret);
        self
    }
    
    /// The language for API responses.
    pub fn language(mut self, language: String) -> Self {
        self.language = language;
        self
    }
    
    /// The [`ClassInfoCache`] to use for this manager. Useful if instantiating multiple managers 
    /// to share state.
    pub fn classinfo_cache(mut self, classinfo_cache: Arc<Mutex<ClassInfoCache>>) -> Self {
        self.classinfo_cache = classinfo_cache;
        self
    }
    
    /// Client to use for requests. Remember to also include the cookies connected to this client
    /// or you will need to set the cookies outside of the module.
    pub fn client(mut self, client: ClientWithMiddleware) -> Self {
        self.client = Some(client);
        self
    }
    
    /// Request cookies.
    pub fn cookies(mut self, cookies: Arc<Jar>) -> Self {
        self.cookies = Some(cookies);
        self
    }
    
    /// User agent for requests. If you provided a client this is not needed as the user agent 
    /// associated with the client is used.
    pub fn user_agent(mut self, user_agent: &'static str) -> Self {
        self.user_agent = user_agent;
        self
    }
    
    /// How many seconds your computer is behind Steam's servers. Used in mobile confirmations.
    pub fn time_offset(mut self, time_offset: i64) -> Self {
        self.time_offset = time_offset;
        self
    }
    
    /// Builds the [`TradeOfferManager`].
    pub fn build(self) -> TradeOfferManager {
        self.into()
    }
}