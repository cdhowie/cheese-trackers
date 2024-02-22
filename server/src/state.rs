//! Server state management.
//!
//! This module provides traits that can be used to obtain specific
//! functionality from the global server state.  This allows better separation
//! of concerns between different server components.

use std::sync::Arc;

use crate::{auth::token::TokenProcessor, db::DataAccessProvider};

/// Provides access to a [`DataAccessProvider`].
pub trait GetDataAccessProvider {
    type DataProvider: DataAccessProvider;

    fn get_data_provider(&self) -> &Self::DataProvider;
}

impl<T: GetDataAccessProvider> GetDataAccessProvider for Arc<T> {
    type DataProvider = T::DataProvider;

    fn get_data_provider(&self) -> &Self::DataProvider {
        (**self).get_data_provider()
    }
}

/// Provides access to a [`TokenProcessor`].
pub trait GetTokenProcessor {
    fn get_token_processor(&self) -> &TokenProcessor;
}

impl<T: GetTokenProcessor> GetTokenProcessor for Arc<T> {
    fn get_token_processor(&self) -> &TokenProcessor {
        (**self).get_token_processor()
    }
}
