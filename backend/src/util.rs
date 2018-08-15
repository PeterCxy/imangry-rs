use actix_web::dev::ConnectionInfo;
use rand::{self, Rng};
use std::str;

/*
 * Generate a random alphanumeric string of a specified length
 */
const DICTIONARY: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ1234567890";
pub fn rand_str(len: usize) -> String {
    let mut rng = rand::thread_rng();
    let mut ret: Vec<u8> = Vec::with_capacity(len);
    for _ in 0..len {
        ret.push(DICTIONARY[rng.gen_range(0, DICTIONARY.len())]);
    }
    str::from_utf8(ret.as_slice()).unwrap().to_string()
}

/*
 * Construct scheme://host from ConnectionInfo
 */
pub fn conn_scheme_host_port(info: &ConnectionInfo) -> String {
    format!("{}://{}", info.scheme(), info.host())
}

/*
 * Return true if the user agent is considered a browser
 */
const BROWSER_AGENTS: &[&str] = &[
    "chrome",
    "msie",
    "trident",
    "firefox",
    "safari",
];
pub fn ua_is_browser(ua: &str) -> bool {
    let ua = ua.to_lowercase();
    for ba in BROWSER_AGENTS {
        if ua.contains(ba) {
            return true;
        }
    }
    return false;
}