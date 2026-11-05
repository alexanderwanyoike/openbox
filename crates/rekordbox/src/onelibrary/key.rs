// Adapted from Baken 8446116b51dba57370c37ff3bcc1efd69ab9ab40, MIT.
// The public device key encoding originates in pyrekordbox (MIT).
const BLOB: &[u8] =
    b"PN_1dH8$oLJY)16j_RvM6qphWw`476>;C1cWmI#se(PG`j}~xAjlufj?`#0i{;=glh(SkW)y0>n?YEiD`l%t(";
const BLOB_KEY: &[u8] = b"657f48f84c437cc1";

pub(super) fn passphrase() -> Result<String, String> {
    const DIGITS: &[u8] =
        b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz!#$%&()*+-;<=>?@^_`{|}~";
    let mut data = Vec::with_capacity(BLOB.len() * 4 / 5 + 4);
    for chunk in BLOB.chunks(5) {
        let mut acc: u32 = 0;
        for i in 0..5 {
            let c = chunk.get(i).copied().unwrap_or(b'~');
            let d = DIGITS
                .iter()
                .position(|&x| x == c)
                .ok_or("Invalid OneLibrary key encoding")?;
            acc = acc.wrapping_mul(85).wrapping_add(d as u32);
        }
        data.extend_from_slice(&acc.to_be_bytes()[..chunk.len() - 1]);
    }
    for (i, b) in data.iter_mut().enumerate() {
        *b ^= BLOB_KEY[i % BLOB_KEY.len()];
    }
    let text = miniz_oxide::inflate::decompress_to_vec_zlib(&data)
        .map_err(|e| format!("Invalid OneLibrary key data: {e:?}"))?;
    String::from_utf8(text).map_err(|e| e.to_string())
}
