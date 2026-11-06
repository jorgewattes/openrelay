//! Share a circuit as a link, like Falstad's `?ctz=…` links.
//!
//! The whole circuit travels inside the link: a compact text form is
//! compressed (deflate) and written in URL-safe base64. Opening the link
//! rebuilds the circuit; editing it later never changes the original link.

use crate::circuit::{Circuit, Kind, P};

/// Public address of the web version (used to build links from the desktop app).
pub const PUBLIC_URL: &str = "https://jorgewattes.github.io/simueletro/";

/// Stable numbering of the parts inside links. Only ever append to this list.
const KINDS: [Kind; 27] = [
    Kind::TermL1,
    Kind::TermL2,
    Kind::TermL3,
    Kind::TermN,
    Kind::TermPE,
    Kind::TermP24,
    Kind::Term0V,
    Kind::Fuse,
    Kind::Breaker1,
    Kind::Breaker3,
    Kind::Thermal3,
    Kind::PushNo,
    Kind::PushNc,
    Kind::Switch,
    Kind::EStop,
    Kind::LimitNo,
    Kind::LimitNc,
    Kind::Coil,
    Kind::TimerOn,
    Kind::TimerOff,
    Kind::ContactNo,
    Kind::ContactNc,
    Kind::Main3,
    Kind::Lamp,
    Kind::Buzzer,
    Kind::Motor1,
    Kind::Motor3,
];

fn kind_index(k: Kind) -> usize {
    KINDS.iter().position(|&x| x == k).unwrap_or(0)
}

/// Compact text: `1~comp;comp…~wire;wire…`, comp = `kind,x,y,rot,tag,preset×10,color,num`.
fn to_text(c: &Circuit) -> String {
    let comps: Vec<String> = c
        .comps
        .iter()
        .map(|p| {
            let tag: String = p.tag.chars().filter(|ch| !",;~".contains(*ch)).collect();
            let mut s = format!("{},{},{},{},{}", kind_index(p.kind), p.pos.x, p.pos.y, p.rot, tag);
            let preset = (p.preset * 10.0).round() as i64;
            if preset != 30 || p.color != 0 || p.num != 0 {
                s.push_str(&format!(",{preset},{},{}", p.color, p.num));
            }
            s
        })
        .collect();
    let wires: Vec<String> = c
        .wires
        .iter()
        .map(|w| format!("{},{},{},{}", w.a.x, w.a.y, w.b.x, w.b.y))
        .collect();
    format!("1~{}~{}", comps.join(";"), wires.join(";"))
}

fn from_text(s: &str) -> Option<Circuit> {
    let mut parts = s.split('~');
    if parts.next()? != "1" {
        return None;
    }
    let comps = parts.next().unwrap_or("");
    let wires = parts.next().unwrap_or("");
    let mut c = Circuit::default();
    for item in comps.split(';').filter(|x| !x.is_empty()) {
        let f: Vec<&str> = item.split(',').collect();
        if f.len() < 5 {
            return None;
        }
        let kind = *KINDS.get(f[0].parse::<usize>().ok()?)?;
        let pos = P::new(f[1].parse().ok()?, f[2].parse().ok()?);
        let rot: u8 = f[3].parse().ok()?;
        let id = c.add(kind, pos, rot % 4, f[4]);
        let comp = c.comp_mut(id)?;
        if f.len() >= 8 {
            comp.preset = (f[5].parse::<i64>().ok()? as f32 / 10.0).clamp(0.1, 600.0);
            comp.color = f[6].parse().ok()?;
            comp.num = f[7].parse().ok()?;
        } else {
            comp.preset = 3.0;
            comp.color = 0;
            comp.num = 0;
        }
    }
    for item in wires.split(';').filter(|x| !x.is_empty()) {
        let v: Vec<i32> = item.split(',').filter_map(|x| x.parse().ok()).collect();
        if v.len() != 4 {
            return None;
        }
        c.wire((v[0], v[1]), (v[2], v[3]));
    }
    Some(c)
}

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

fn b64_encode(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len() * 4 / 3 + 4);
    for chunk in data.chunks(3) {
        let b = [chunk[0], *chunk.get(1).unwrap_or(&0), *chunk.get(2).unwrap_or(&0)];
        let n = (b[0] as u32) << 16 | (b[1] as u32) << 8 | b[2] as u32;
        let chars = chunk.len() + 1;
        for i in 0..chars {
            out.push(B64[((n >> (18 - 6 * i)) & 63) as usize] as char);
        }
    }
    out
}

fn b64_decode(s: &str) -> Option<Vec<u8>> {
    let val = |c: u8| -> Option<u32> {
        match c {
            b'A'..=b'Z' => Some((c - b'A') as u32),
            b'a'..=b'z' => Some((c - b'a' + 26) as u32),
            b'0'..=b'9' => Some((c - b'0' + 52) as u32),
            b'-' | b'+' => Some(62),
            b'_' | b'/' => Some(63),
            _ => None,
        }
    };
    let bytes: Vec<u8> = s.bytes().filter(|c| *c != b'=').collect();
    let mut out = Vec::with_capacity(bytes.len() * 3 / 4);
    for chunk in bytes.chunks(4) {
        if chunk.len() < 2 {
            return None;
        }
        let mut n = 0u32;
        for (i, &c) in chunk.iter().enumerate() {
            n |= val(c)? << (18 - 6 * i);
        }
        out.push((n >> 16) as u8);
        if chunk.len() > 2 {
            out.push((n >> 8) as u8);
        }
        if chunk.len() > 3 {
            out.push(n as u8);
        }
    }
    Some(out)
}

/// Circuit → short token for the link.
pub fn encode(c: &Circuit) -> String {
    let packed = miniz_oxide::deflate::compress_to_vec(to_text(c).as_bytes(), 10);
    b64_encode(&packed)
}

/// Token → circuit (None if the link is damaged).
pub fn decode(token: &str) -> Option<Circuit> {
    let packed = b64_decode(token.trim())?;
    let raw = miniz_oxide::inflate::decompress_to_vec_with_limit(&packed, 4 << 20).ok()?;
    from_text(std::str::from_utf8(&raw).ok()?)
}

/// Full link for a circuit, based on `base` (the page address).
pub fn link(c: &Circuit, base: &str) -> String {
    let base = base.split(['?', '#']).next().unwrap_or(base);
    format!("{base}?c={}", encode(c))
}

/// Reads `c=…` from a link (query or fragment) and rebuilds the circuit.
pub fn decode_from_url(url: &str) -> Option<Circuit> {
    let start = url.find(['?', '#'])?;
    url[start + 1..]
        .split(['&', '?', '#'])
        .find_map(|kv| kv.strip_prefix("c="))
        .and_then(decode)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::examples;

    #[test]
    fn examples_round_trip() {
        for (_, f) in examples::list() {
            let c = f();
            let url = link(&c, PUBLIC_URL);
            let back = decode_from_url(&url).expect("decodes");
            assert_eq!(back.comps.len(), c.comps.len());
            assert_eq!(back.wires, c.wires);
            for (a, b) in c.comps.iter().zip(&back.comps) {
                assert_eq!((a.kind, a.pos, a.rot, &a.tag, a.color, a.num), (b.kind, b.pos, b.rot, &b.tag, b.color, b.num));
                assert!((a.preset - b.preset).abs() < 0.05);
            }
            assert!(url.len() < 1200, "link too long: {}", url.len());
        }
    }

    /// `cargo test print_example_link -- --ignored --nocapture` prints a sample link.
    #[test]
    #[ignore]
    fn print_example_link() {
        println!("LINK {}", link(&examples::reversing(), PUBLIC_URL));
    }

    #[test]
    fn damaged_link_is_rejected() {
        assert!(decode_from_url("https://x/?c=@@@").is_none());
        assert!(decode_from_url("https://x/").is_none());
    }
}
