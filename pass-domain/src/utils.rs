use rand::Rng;

pub fn random_string(length: usize) -> String {
    let chars = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789".as_bytes();

    let mut res = String::new();
    while res.len() < length {
        let idx = rand::rng().random_range(0..chars.len());
        res.push(chars[idx] as char);
    }

    res
}
