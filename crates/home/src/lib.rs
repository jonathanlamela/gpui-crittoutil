//! The Home screen's feature-search logic, plus the `Route` enum it resolves
//! queries to. `Route` lives here (rather than in the `app` crate) because
//! the Home screen is what actually needs to name every navigable screen to
//! score a search query against it; the `app` crate re-exports it as the
//! single source of truth for navigation across the whole app.

mod home_search;

pub use home_search::search;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Route {
    Home,
    Converter,
    KeyGenerator,
    Encrypter,
    Decrypter,
    FileHasher,
}

impl Route {
    pub const ALL: [Route; 6] = [
        Route::Home,
        Route::Converter,
        Route::KeyGenerator,
        Route::Encrypter,
        Route::Decrypter,
        Route::FileHasher,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            Route::Home => "Home",
            Route::Converter => "Converter",
            Route::KeyGenerator => "Key generator",
            Route::Encrypter => "Encrypter",
            Route::Decrypter => "Decrypter",
            Route::FileHasher => "File hasher",
        }
    }
}
