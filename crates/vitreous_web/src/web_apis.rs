use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestInit, RequestMode, Response, Storage, Window};

/// Error type for web API operations.
#[derive(Debug)]
pub enum WebError {
    /// A JavaScript exception was thrown.
    Js(JsValue),
    /// The requested browser API is not available.
    Unavailable(&'static str),
}

impl From<JsValue> for WebError {
    fn from(v: JsValue) -> Self {
        WebError::Js(v)
    }
}

impl std::fmt::Display for WebError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WebError::Js(v) => write!(f, "JS error: {v:?}"),
            WebError::Unavailable(api) => write!(f, "{api} is not available"),
        }
    }
}

fn window() -> Result<Window, WebError> {
    web_sys::window().ok_or(WebError::Unavailable("window"))
}

// ---------------------------------------------------------------------------
// Fetch API
// ---------------------------------------------------------------------------

/// Response from a `fetch()` call.
pub struct FetchResponse {
    response: Response,
}

impl FetchResponse {
    /// HTTP status code.
    pub fn status(&self) -> u16 {
        self.response.status()
    }

    /// Whether the response was successful (status 200-299).
    pub fn ok(&self) -> bool {
        self.response.ok()
    }

    /// Read the response body as text.
    pub async fn text(&self) -> Result<String, WebError> {
        let promise = self.response.text()?;
        let value = JsFuture::from(promise).await?;
        Ok(value.as_string().unwrap_or_default())
    }

    /// Read the response body as JSON (returns a `JsValue`).
    pub async fn json(&self) -> Result<JsValue, WebError> {
        let promise = self.response.json()?;
        let value = JsFuture::from(promise).await?;
        Ok(value)
    }
}

/// Perform an HTTP fetch request.
///
/// # Example
///
/// ```ignore
/// let response = vitreous_web::web_apis::fetch("https://api.example.com/data").await?;
/// let body = response.text().await?;
/// ```
pub async fn fetch(url: &str) -> Result<FetchResponse, WebError> {
    let w = window()?;
    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let request = Request::new_with_str_and_init(url, &opts)?;

    let promise = w.fetch_with_request(&request);
    let resp_value = JsFuture::from(promise).await?;
    let response: Response = resp_value.dyn_into()?;

    Ok(FetchResponse { response })
}

/// Perform an HTTP fetch with a custom method, headers, and optional body.
pub async fn fetch_with_options(
    url: &str,
    method: &str,
    headers: &[(&str, &str)],
    body: Option<&str>,
) -> Result<FetchResponse, WebError> {
    let w = window()?;
    let opts = RequestInit::new();
    opts.set_method(method);
    opts.set_mode(RequestMode::Cors);

    if let Some(b) = body {
        opts.set_body(&JsValue::from_str(b));
    }

    let request = Request::new_with_str_and_init(url, &opts)?;

    let req_headers = request.headers();
    for (k, v) in headers {
        req_headers.set(k, v)?;
    }

    let promise = w.fetch_with_request(&request);
    let resp_value = JsFuture::from(promise).await?;
    let response: Response = resp_value.dyn_into()?;

    Ok(FetchResponse { response })
}

// ---------------------------------------------------------------------------
// Local Storage
// ---------------------------------------------------------------------------

/// Access to the browser's `localStorage`.
pub struct LocalStorage {
    storage: Storage,
}

/// Get a handle to `localStorage`.
pub fn local_storage() -> Result<LocalStorage, WebError> {
    let w = window()?;
    let storage = w
        .local_storage()
        .map_err(WebError::Js)?
        .ok_or(WebError::Unavailable("localStorage"))?;
    Ok(LocalStorage { storage })
}

impl LocalStorage {
    /// Get a value by key.
    pub fn get(&self, key: &str) -> Option<String> {
        self.storage.get_item(key).ok().flatten()
    }

    /// Set a key-value pair.
    pub fn set(&self, key: &str, value: &str) -> Result<(), WebError> {
        self.storage.set_item(key, value).map_err(WebError::Js)
    }

    /// Remove a key.
    pub fn remove(&self, key: &str) -> Result<(), WebError> {
        self.storage.remove_item(key).map_err(WebError::Js)
    }

    /// Clear all stored data.
    pub fn clear(&self) -> Result<(), WebError> {
        self.storage.clear().map_err(WebError::Js)
    }

    /// Number of stored items.
    pub fn len(&self) -> u32 {
        self.storage.length().unwrap_or(0)
    }

    /// Whether storage is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

// ---------------------------------------------------------------------------
// Location / Navigation (History API)
// ---------------------------------------------------------------------------

/// Get the current browser URL path (e.g., `/users/42`).
pub fn current_path() -> Result<String, WebError> {
    let w = window()?;
    let location = w.location();
    location.pathname().map_err(WebError::Js)
}

/// Get the full current URL as a `Location`-like struct.
pub fn location() -> Result<LocationInfo, WebError> {
    let w = window()?;
    let loc = w.location();
    Ok(LocationInfo {
        href: loc.href().unwrap_or_default(),
        pathname: loc.pathname().unwrap_or_default(),
        search: loc.search().unwrap_or_default(),
        hash: loc.hash().unwrap_or_default(),
        host: loc.host().unwrap_or_default(),
        hostname: loc.hostname().unwrap_or_default(),
        protocol: loc.protocol().unwrap_or_default(),
        port: loc.port().unwrap_or_default(),
    })
}

/// Browser location information.
#[derive(Debug, Clone)]
pub struct LocationInfo {
    pub href: String,
    pub pathname: String,
    pub search: String,
    pub hash: String,
    pub host: String,
    pub hostname: String,
    pub protocol: String,
    pub port: String,
}

/// Navigate to a new URL path using the History API (no page reload).
///
/// # Example
///
/// ```ignore
/// vitreous_web::web_apis::navigate("/users/42");
/// ```
pub fn navigate(path: &str) -> Result<(), WebError> {
    let w = window()?;
    let history = w.history().map_err(WebError::Js)?;
    history
        .push_state_with_url(&JsValue::NULL, "", Some(path))
        .map_err(WebError::Js)
}

/// Listen for browser navigation events (back/forward buttons).
///
/// The provided callback is called with the new path whenever a `popstate`
/// event fires.
///
/// Returns a closure guard — dropping it removes the listener.
pub fn on_navigate(callback: impl Fn(String) + 'static) -> Result<NavigateGuard, WebError> {
    let w = window()?;

    let closure = Closure::new(move |_: web_sys::Event| {
        if let Ok(path) = current_path() {
            callback(path);
        }
    });

    w.add_event_listener_with_callback("popstate", closure.as_ref().unchecked_ref())
        .map_err(WebError::Js)?;

    Ok(NavigateGuard { _closure: closure })
}

/// Guard that keeps a `popstate` event listener alive.
/// Dropping this removes the listener.
pub struct NavigateGuard {
    _closure: Closure<dyn Fn(web_sys::Event)>,
}
