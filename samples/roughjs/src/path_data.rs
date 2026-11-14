//! A port of `path-data-parser` 0.1.0: SVG path data parsing,
//! `absolutize`, and `normalize` to M, L, C and Z.

use crate::core::Point;
use crate::js;

#[derive(Debug, Clone, PartialEq)]
pub struct Segment {
    pub key: char,
    pub data: Vec<f64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathDataError {
    /// The tokenizer met a character it does not know. In JavaScript this
    /// surfaces as a `TypeError` when the parser reads the missing token.
    BadCharacter,
    ParamNotANumber,
    EndedShort,
    /// A number follows a command that takes no parameters, such as `Z`.
    /// path-data-parser loops forever on this input; the port stops instead.
    NumberAfterParameterlessCommand,
}

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Command(char),
    Number(f64),
    End,
}

fn params(command: char) -> usize {
    match command {
        'A' | 'a' => 7,
        'C' | 'c' => 6,
        'H' | 'h' | 'V' | 'v' => 1,
        'L' | 'l' | 'M' | 'm' | 'T' | 't' => 2,
        'Q' | 'q' | 'S' | 's' => 4,
        _ => 0, // Z, z
    }
}

/// Matches `/^(([-+]?[0-9]+(\.[0-9]*)?|[-+]?\.[0-9]+)([eE][-+]?[0-9]+)?)/`
/// and returns the matched length.
fn match_number(s: &[u8]) -> Option<usize> {
    let digits = |from: usize| s[from..].iter().take_while(|b| b.is_ascii_digit()).count();
    let mut i = usize::from(matches!(s.first(), Some(b'+' | b'-')));
    let int_digits = digits(i);
    if int_digits > 0 {
        i += int_digits;
        if s.get(i) == Some(&b'.') {
            i += 1 + digits(i + 1);
        }
    } else {
        if s.get(i) != Some(&b'.') {
            return None;
        }
        let frac_digits = digits(i + 1);
        if frac_digits == 0 {
            return None;
        }
        i += 1 + frac_digits;
    }
    if matches!(s.get(i), Some(b'e' | b'E')) {
        let mut j = i + 1;
        if matches!(s.get(j), Some(b'+' | b'-')) {
            j += 1;
        }
        let exp_digits = digits(j);
        if exp_digits > 0 {
            i = j + exp_digits;
        }
    }
    Some(i)
}

fn tokenize(d: &str) -> Option<Vec<Token>> {
    let bytes = d.as_bytes();
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let rest = &bytes[i..];
        let spaces = rest
            .iter()
            .take_while(|b| matches!(b, b' ' | b'\t' | b'\r' | b'\n' | b','))
            .count();
        if spaces > 0 {
            i += spaces;
        } else if b"aAcChHlLmMqQsStTvVzZ".contains(&rest[0]) {
            tokens.push(Token::Command(char::from(rest[0])));
            i += 1;
        } else if let Some(len) = match_number(rest) {
            let text = std::str::from_utf8(&rest[..len]).expect("ascii");
            tokens.push(Token::Number(js::parse_float(text)));
            i += len;
        } else {
            return None;
        }
    }
    tokens.push(Token::End);
    Some(tokens)
}

/// `parsePath`.
pub fn parse_path(d: &str) -> Result<Vec<Segment>, PathDataError> {
    let mut segments = Vec::new();
    let tokens = tokenize(d).ok_or(PathDataError::BadCharacter)?;
    let mut mode: Option<char> = None; // None is 'BOD'
    let mut index = 0;
    let mut token = &tokens[index];
    while *token != Token::End {
        let params_count;
        match (mode, token) {
            (None, Token::Command(c @ ('M' | 'm'))) => {
                index += 1;
                params_count = params(*c);
                mode = Some(*c);
            }
            (None, _) => return parse_path(&format!("M0,0{d}")),
            (Some(m), Token::Number(_)) => {
                params_count = params(m);
                if params_count == 0 {
                    return Err(PathDataError::NumberAfterParameterlessCommand);
                }
            }
            (Some(_), Token::Command(c)) => {
                index += 1;
                params_count = params(*c);
                mode = Some(*c);
            }
            (Some(_), Token::End) => unreachable!(),
        }
        if index + params_count < tokens.len() {
            let mut data = Vec::with_capacity(params_count);
            for token in &tokens[index..index + params_count] {
                match token {
                    Token::Number(n) => data.push(*n),
                    _ => return Err(PathDataError::ParamNotANumber),
                }
            }
            let key = mode.expect("mode");
            segments.push(Segment { key, data });
            index += params_count;
            token = &tokens[index];
            if key == 'M' {
                mode = Some('L');
            }
            if key == 'm' {
                mode = Some('l');
            }
        } else {
            return Err(PathDataError::EndedShort);
        }
    }
    Ok(segments)
}

/// `absolutize`: relative commands become absolute.
pub fn absolutize(segments: &[Segment]) -> Vec<Segment> {
    let (mut cx, mut cy) = (0.0, 0.0);
    let (mut subx, mut suby) = (0.0, 0.0);
    let mut out = Vec::new();
    let seg = |key: char, data: Vec<f64>| Segment { key, data };
    let relative = |data: &[f64], cx: f64, cy: f64| -> Vec<f64> {
        data.iter()
            .enumerate()
            .map(|(i, d)| if i % 2 == 1 { d + cy } else { d + cx })
            .collect()
    };
    for Segment { key, data } in segments {
        match key {
            'M' => {
                out.push(seg('M', data.clone()));
                (cx, cy) = (data[0], data[1]);
                (subx, suby) = (data[0], data[1]);
            }
            'm' => {
                cx += data[0];
                cy += data[1];
                out.push(seg('M', vec![cx, cy]));
                subx = cx;
                suby = cy;
            }
            'L' => {
                out.push(seg('L', data.clone()));
                (cx, cy) = (data[0], data[1]);
            }
            'l' => {
                cx += data[0];
                cy += data[1];
                out.push(seg('L', vec![cx, cy]));
            }
            'C' => {
                out.push(seg('C', data.clone()));
                cx = data[4];
                cy = data[5];
            }
            'c' => {
                let new = relative(data, cx, cy);
                cx = new[4];
                cy = new[5];
                out.push(seg('C', new));
            }
            'Q' => {
                out.push(seg('Q', data.clone()));
                cx = data[2];
                cy = data[3];
            }
            'q' => {
                let new = relative(data, cx, cy);
                cx = new[2];
                cy = new[3];
                out.push(seg('Q', new));
            }
            'A' => {
                out.push(seg('A', data.clone()));
                cx = data[5];
                cy = data[6];
            }
            'a' => {
                cx += data[5];
                cy += data[6];
                out.push(seg(
                    'A',
                    vec![data[0], data[1], data[2], data[3], data[4], cx, cy],
                ));
            }
            'H' => {
                out.push(seg('H', data.clone()));
                cx = data[0];
            }
            'h' => {
                cx += data[0];
                out.push(seg('H', vec![cx]));
            }
            'V' => {
                out.push(seg('V', data.clone()));
                cy = data[0];
            }
            'v' => {
                cy += data[0];
                out.push(seg('V', vec![cy]));
            }
            'S' => {
                out.push(seg('S', data.clone()));
                cx = data[2];
                cy = data[3];
            }
            's' => {
                let new = relative(data, cx, cy);
                cx = new[2];
                cy = new[3];
                out.push(seg('S', new));
            }
            'T' => {
                out.push(seg('T', data.clone()));
                cx = data[0];
                cy = data[1];
            }
            't' => {
                cx += data[0];
                cy += data[1];
                out.push(seg('T', vec![cx, cy]));
            }
            'Z' | 'z' => {
                out.push(seg('Z', vec![]));
                cx = subx;
                cy = suby;
            }
            _ => {}
        }
    }
    out
}

/// `normalize`: only M, L, C and Z remain.
pub fn normalize(segments: &[Segment]) -> Vec<Segment> {
    let mut out = Vec::new();
    let mut last_type = ' ';
    let (mut cx, mut cy) = (0.0, 0.0);
    let (mut subx, mut suby) = (0.0, 0.0);
    let (mut lcx, mut lcy) = (0.0, 0.0);
    let seg = |key: char, data: Vec<f64>| Segment { key, data };
    for Segment { key, data } in segments {
        match key {
            'M' => {
                out.push(seg('M', data.clone()));
                (cx, cy) = (data[0], data[1]);
                (subx, suby) = (data[0], data[1]);
            }
            'C' => {
                out.push(seg('C', data.clone()));
                cx = data[4];
                cy = data[5];
                lcx = data[2];
                lcy = data[3];
            }
            'L' => {
                out.push(seg('L', data.clone()));
                (cx, cy) = (data[0], data[1]);
            }
            'H' => {
                cx = data[0];
                out.push(seg('L', vec![cx, cy]));
            }
            'V' => {
                cy = data[0];
                out.push(seg('L', vec![cx, cy]));
            }
            'S' => {
                let (cx1, cy1) = if last_type == 'C' || last_type == 'S' {
                    (cx + (cx - lcx), cy + (cy - lcy))
                } else {
                    (cx, cy)
                };
                let mut d = vec![cx1, cy1];
                d.extend_from_slice(data);
                out.push(seg('C', d));
                lcx = data[0];
                lcy = data[1];
                cx = data[2];
                cy = data[3];
            }
            'T' => {
                let (x, y) = (data[0], data[1]);
                let (x1, y1) = if last_type == 'Q' || last_type == 'T' {
                    (cx + (cx - lcx), cy + (cy - lcy))
                } else {
                    (cx, cy)
                };
                let cx1 = cx + 2.0 * (x1 - cx) / 3.0;
                let cy1 = cy + 2.0 * (y1 - cy) / 3.0;
                let cx2 = x + 2.0 * (x1 - x) / 3.0;
                let cy2 = y + 2.0 * (y1 - y) / 3.0;
                out.push(seg('C', vec![cx1, cy1, cx2, cy2, x, y]));
                lcx = x1;
                lcy = y1;
                cx = x;
                cy = y;
            }
            'Q' => {
                let (x1, y1, x, y) = (data[0], data[1], data[2], data[3]);
                let cx1 = cx + 2.0 * (x1 - cx) / 3.0;
                let cy1 = cy + 2.0 * (y1 - cy) / 3.0;
                let cx2 = x + 2.0 * (x1 - x) / 3.0;
                let cy2 = y + 2.0 * (y1 - y) / 3.0;
                out.push(seg('C', vec![cx1, cy1, cx2, cy2, x, y]));
                lcx = x1;
                lcy = y1;
                cx = x;
                cy = y;
            }
            'A' => {
                let r1 = data[0].abs();
                let r2 = data[1].abs();
                let angle = data[2];
                let large_arc_flag = data[3];
                let sweep_flag = data[4];
                let x = data[5];
                let y = data[6];
                if r1 == 0.0 || r2 == 0.0 {
                    out.push(seg('C', vec![cx, cy, x, y, x, y]));
                    cx = x;
                    cy = y;
                } else if cx != x || cy != y {
                    let curves = arc_to_cubic_curves(
                        cx,
                        cy,
                        x,
                        y,
                        r1,
                        r2,
                        angle,
                        large_arc_flag,
                        sweep_flag,
                    );
                    for curve in curves {
                        out.push(seg('C', curve.to_vec()));
                    }
                    cx = x;
                    cy = y;
                }
            }
            'Z' => {
                out.push(seg('Z', vec![]));
                cx = subx;
                cy = suby;
            }
            _ => {}
        }
        last_type = *key;
    }
    out
}

fn deg_to_rad(degrees: f64) -> f64 {
    (std::f64::consts::PI * degrees) / 180.0
}

fn rotate(x: f64, y: f64, angle_rad: f64) -> Point {
    let big_x = x * js::cos(angle_rad) - y * js::sin(angle_rad);
    let big_y = x * js::sin(angle_rad) + y * js::cos(angle_rad);
    [big_x, big_y]
}

#[allow(clippy::too_many_arguments)]
fn arc_to_cubic_curves(
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
    r1: f64,
    r2: f64,
    angle: f64,
    large_arc_flag: f64,
    sweep_flag: f64,
) -> Vec<[f64; 6]> {
    let angle_rad = deg_to_rad(angle);
    let params = arc_points(
        x1,
        y1,
        x2,
        y2,
        r1,
        r2,
        angle,
        large_arc_flag,
        sweep_flag,
        None,
    );
    params
        .chunks(3)
        .map(|p| {
            let r1 = rotate(p[0][0], p[0][1], angle_rad);
            let r2 = rotate(p[1][0], p[1][1], angle_rad);
            let r3 = rotate(p[2][0], p[2][1], angle_rad);
            [r1[0], r1[1], r2[0], r2[1], r3[0], r3[1]]
        })
        .collect()
}

/// The recursive part of `arcToCubicCurves`, returning control points.
#[allow(clippy::too_many_arguments)]
fn arc_points(
    mut x1: f64,
    mut y1: f64,
    mut x2: f64,
    mut y2: f64,
    mut r1: f64,
    mut r2: f64,
    angle: f64,
    large_arc_flag: f64,
    sweep_flag: f64,
    recursive: Option<[f64; 4]>,
) -> Vec<Point> {
    use std::f64::consts::PI;
    let angle_rad = deg_to_rad(angle);
    let mut params = Vec::new();
    let (mut f1, mut f2, cx, cy);
    if let Some([a, b, c, d]) = recursive {
        (f1, f2, cx, cy) = (a, b, c, d);
    } else {
        [x1, y1] = rotate(x1, y1, -angle_rad);
        [x2, y2] = rotate(x2, y2, -angle_rad);
        let x = (x1 - x2) / 2.0;
        let y = (y1 - y2) / 2.0;
        let mut h = (x * x) / (r1 * r1) + (y * y) / (r2 * r2);
        if h > 1.0 {
            h = h.sqrt();
            r1 *= h;
            r2 *= h;
        }
        let sign = if large_arc_flag == sweep_flag {
            -1.0
        } else {
            1.0
        };
        let r1_pow = r1 * r1;
        let r2_pow = r2 * r2;
        let left = r1_pow * r2_pow - r1_pow * y * y - r2_pow * x * x;
        let right = r1_pow * y * y + r2_pow * x * x;
        let k = sign * (left / right).abs().sqrt();
        cx = k * r1 * y / r2 + (x1 + x2) / 2.0;
        cy = k * -r2 * x / r1 + (y1 + y2) / 2.0;
        f1 = js::asin(js::parse_float(&js::to_fixed((y1 - cy) / r2, 9)));
        f2 = js::asin(js::parse_float(&js::to_fixed((y2 - cy) / r2, 9)));
        if x1 < cx {
            f1 = PI - f1;
        }
        if x2 < cx {
            f2 = PI - f2;
        }
        if f1 < 0.0 {
            f1 += PI * 2.0;
        }
        if f2 < 0.0 {
            f2 += PI * 2.0;
        }
        if js::truthy(sweep_flag) && f1 > f2 {
            f1 -= PI * 2.0;
        }
        if !js::truthy(sweep_flag) && f2 > f1 {
            f2 -= PI * 2.0;
        }
    }
    let mut df = f2 - f1;
    if df.abs() > (PI * 120.0 / 180.0) {
        let f2old = f2;
        let x2old = x2;
        let y2old = y2;
        if js::truthy(sweep_flag) && f2 > f1 {
            f2 = f1 + (PI * 120.0 / 180.0) * 1.0;
        } else {
            f2 = f1 - (PI * 120.0 / 180.0);
        }
        x2 = cx + r1 * js::cos(f2);
        y2 = cy + r2 * js::sin(f2);
        params = arc_points(
            x2,
            y2,
            x2old,
            y2old,
            r1,
            r2,
            angle,
            0.0,
            sweep_flag,
            Some([f2, f2old, cx, cy]),
        );
    }
    df = f2 - f1;
    let c1 = js::cos(f1);
    let s1 = js::sin(f1);
    let c2 = js::cos(f2);
    let s2 = js::sin(f2);
    let t = js::tan(df / 4.0);
    let hx = 4.0 / 3.0 * r1 * t;
    let hy = 4.0 / 3.0 * r2 * t;
    let m1 = [x1, y1];
    let mut m2 = [x1 + hx * s1, y1 - hy * c1];
    let m3 = [x2 + hx * s2, y2 - hy * c2];
    let m4 = [x2, y2];
    m2[0] = 2.0 * m1[0] - m2[0];
    m2[1] = 2.0 * m1[1] - m2[1];
    let mut out = vec![m2, m3, m4];
    out.extend(params);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_like_path_data_parser() {
        let segments = parse_path("m10 10h5v-5z").unwrap();
        let keys: String = segments.iter().map(|s| s.key).collect();
        assert_eq!(keys, "mhvz");
        let abs = absolutize(&segments);
        assert_eq!(abs[1].data, vec![15.0]);
        assert_eq!(abs[2].data, vec![5.0]);
        let norm = normalize(&abs);
        assert_eq!(norm[2].data, vec![15.0, 5.0]);
    }

    #[test]
    fn implicit_moveto_and_numbers() {
        // "1.5.5" is two numbers, and "1e" stops before the "e".
        assert_eq!(parse_path("M1.5.5").unwrap()[0].data, vec![1.5, 0.5]);
        assert_eq!(parse_path("L1 2").unwrap()[0].key, 'M');
        assert_eq!(
            parse_path("M0 0 Z 5"),
            Err(PathDataError::NumberAfterParameterlessCommand)
        );
        assert_eq!(parse_path("M0 0 #"), Err(PathDataError::BadCharacter));
        assert_eq!(parse_path("M0"), Err(PathDataError::EndedShort));
    }
}
