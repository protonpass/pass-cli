/*
 *  Copyright (c) 2026 Proton AG
 *  This file is part of Proton AG and Proton Pass.
 *
 *  Proton Pass is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU General Public License as published by
 *  the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  Proton Pass is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 *  GNU General Public License for more details.
 *
 *  You should have received a copy of the GNU General Public License
 *  along with Proton Pass.  If not, see <https://www.gnu.org/licenses/>.
 *
 */

use rand::RngExt;

pub fn random_string(length: usize) -> String {
    let chars = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789".as_bytes();

    let mut res = String::new();
    while res.len() < length {
        let idx = rand::rng().random_range(0..chars.len());
        res.push(chars[idx] as char);
    }

    res
}

pub fn xor_key(key: &[u8], xor_key: u8) -> Vec<u8> {
    let mut res = Vec::with_capacity(key.len());
    for b in key {
        res.push(xor_key ^ b);
    }
    res
}

pub fn xor_key_multibyte(key: &[u8], xor_key: &[u8]) -> Vec<u8> {
    if xor_key.is_empty() {
        return key.to_vec();
    }

    let mut res = Vec::with_capacity(key.len());
    for (i, b) in key.iter().enumerate() {
        res.push(xor_key[i % xor_key.len()] ^ b);
    }
    res
}
