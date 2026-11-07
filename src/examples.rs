//! Ready-made circuits shown in Arquivo → Exemplos.

use crate::circuit::{t, Circuit, Kind, Kind::*, Wire, P};

type Ex = (&'static str, fn() -> Circuit);

/// Examples in groups, numbered in order (shown in Arquivo → Exemplos).
pub fn groups() -> Vec<(&'static str, Vec<Ex>)> {
    vec![
        (
            t("Motores trifásicos", "Three-phase motors"),
            vec![
                (t("1. Partida direta – motor de 3 terminais (estrela interna)", "1. Direct start – 3-terminal motor (internal star)"), direct_start as fn() -> Circuit),
                (t("2. Partida direta – motor de 6 terminais em estrela (Y)", "2. Direct start – 6-terminal motor in star (Y)"), direct_star6),
                (t("3. Partida direta – motor de 6 terminais em triângulo (Δ)", "3. Direct start – 6-terminal motor in delta (Δ)"), direct_delta6),
                (t("4. Reversão – intertravamento por contatos auxiliares", "4. Reversing – interlock by auxiliary contacts"), reversing),
                (t("5. Reversão – intertravamento duplo (botões e contatos)", "5. Reversing – double interlock (buttons and contacts)"), reversing_double),
                (t("6. Partida sequencial temporizada de dois motores", "6. Timed sequential start of two motors"), cascade),
                (t("7. Partida estrela-triângulo temporizada", "7. Timed star-delta start"), star_delta),
            ],
        ),
        (
            t("Motores monofásicos", "Single-phase motors"),
            vec![
                (t("8. Partida direta – monofásico com bobina auxiliar", "8. Direct start – single-phase with start winding"), single_direct as fn() -> Circuit),
                (t("9. Reversão – monofásico com bobina auxiliar", "9. Reversing – single-phase with start winding"), single_reversing),
            ],
        ),
        (
            t("Motores especiais", "Special motors"),
            vec![
                (t("10. Motor Dahlander – duas velocidades (manual)", "10. Dahlander motor – two speeds (manual)"), dahlander as fn() -> Circuit),
                (t("11. Motor de 12 terminais – ΔΔ paralelo (220 V)", "11. 12-terminal motor – ΔΔ parallel (220 V)"), m12_dd),
                (t("12. Motor de 12 terminais – YY paralelo (380 V)", "12. 12-terminal motor – YY parallel (380 V)"), m12_yy),
                (t("13. Motor de 12 terminais – Δ série (440 V)", "13. 12-terminal motor – Δ series (440 V)"), m12_d),
                (t("14. Motor de 12 terminais – Y série (760 V)", "14. 12-terminal motor – Y series (760 V)"), m12_y),
                (t("15. Duas velocidades – motor de dois enrolamentos (manual)", "15. Two speeds – two-winding motor (manual)"), two_winding),
                (t("16. Motor com freio eletromagnético (retificador)", "16. Motor with electromagnetic brake (rectifier)"), brake_motor),
            ],
        ),
        (
            t("Outros", "Others"),
            vec![
                (t("Lâmpada temporizada", "Timed lamp"), timed_lamp as fn() -> Circuit),
                (t("Tensões: motor Y/Δ e lâmpadas", "Voltages: Y/Δ motor and lamps"), voltages),
            ],
        ),
    ]
}

/// All examples, in menu order.
pub fn list() -> Vec<Ex> {
    groups().into_iter().flat_map(|(_, v)| v).collect()
}

// ---------- building blocks ----------

/// Supply terminals L1 L2 L3 (and PE when `pe` is Some(x)) on row y.
fn supply3(c: &mut Circuit, x: i32, y: i32, pe: Option<i32>, sys: u8) {
    for (i, k) in [TermL1, TermL2, TermL3].into_iter().enumerate() {
        let id = c.add(k, (x + 3 * i as i32, y).into(), 0, "");
        c.comp_mut(id).unwrap().volt = sys;
    }
    if let Some(px) = pe {
        c.add(TermPE, (px, y).into(), 0, "");
    }
}

/// Three vertical wires (x, x+3, x+6) from y0 to y1.
fn col3(c: &mut Circuit, x: i32, y0: i32, y1: i32) {
    for k in 0..3 {
        c.wire((x + 3 * k, y0), (x + 3 * k, y1));
    }
}

/// Polyline of wires through the given points.
fn path(c: &mut Circuit, pts: &[(i32, i32)]) {
    for w in pts.windows(2) {
        c.wire(w[0], w[1]);
    }
}

/// Start/stop control with holding contact: L1 at (x,y), fuse F1, stop S0,
/// start S1 with `tag` holding contact, the `tag` coil, N at the bottom.
fn start_stop(c: &mut Circuit, x: i32, y: i32, tag: &str) {
    c.add(TermL1, (x, y).into(), 0, "");
    c.add(Fuse, (x, y + 2).into(), 0, "F1");
    c.wire((x, y), (x, y + 2));
    c.wire((x, y + 8), (x, y + 10));
    c.add(PushNc, (x, y + 10).into(), 0, "S0");
    c.wire((x, y + 16), (x, y + 19));
    c.add(PushNo, (x, y + 19).into(), 0, "S1");
    path(c, &[(x, y + 17), (x + 6, y + 17), (x + 6, y + 19)]);
    c.add(ContactNo, (x + 6, y + 19).into(), 0, tag);
    path(c, &[(x + 6, y + 25), (x + 6, y + 27), (x, y + 27)]);
    c.wire((x, y + 25), (x, y + 29));
    c.add(Coil, (x, y + 29).into(), 0, tag);
    c.wire((x, y + 35), (x, y + 38));
    c.add(TermN, (x, y + 38).into(), 2, "");
}

/// Three-phase power line: L1-L3 + PE at row 4, motor breaker Q1 and contactor
/// `tag`; returns the y where the contactor outputs end.
fn power_line(c: &mut Circuit, x: i32, tag: &str, pe_x: i32) -> i32 {
    supply3(c, x, 4, Some(pe_x), 0);
    c.add(MotorBreaker3, (x, 8).into(), 0, "Q1");
    col3(c, x, 4, 8);
    c.add(Main3, (x, 18).into(), 0, tag);
    col3(c, x, 14, 18);
    24
}

/// Keep tags and detour wires clear of the page frame on the left.
fn fit_left(c: &mut Circuit) {
    let comp_min = c.comps.iter().map(|k| k.pos.x).min().unwrap_or(14);
    let wire_min = c.wires.iter().map(|w| w.a.x.min(w.b.x)).min().unwrap_or(10);
    let dx = (14 - comp_min).max(10 - wire_min).max(0);
    shift_left_part(c, dx);
}
// ---------- three-phase ----------

/// 2: 6-terminal motor (Δ220/Y380 V on 380 V) closed in star.
fn direct_star6() -> Circuit {
    let mut c = Circuit::default();
    let y = power_line(&mut c, 10, "KM1", 19);
    col3(&mut c, 10, y, 32);
    let m = c.add(Motor6X, (10, 32).into(), 0, "M1");
    c.comp_mut(m).unwrap().volt = 0;
    c.wire((19, 4), (19, 32));
    // Star: W2 U2 V2 tied together.
    path(&mut c, &[(10, 43), (13, 43), (16, 43)]);
    start_stop(&mut c, 45, 4, "KM1");
    fit_left(&mut c);
    c
}

/// 3: 6-terminal motor (Δ380/Y660 V on 380 V) closed in delta: each line also
/// feeds the "2" terminal under the next winding (U1-W2, V1-U2, W1-V2).
fn direct_delta6() -> Circuit {
    let mut c = Circuit::default();
    let y = power_line(&mut c, 12, "KM1", 21);
    col3(&mut c, 12, y, 34);
    let m = c.add(Motor6X, (12, 34).into(), 0, "M1");
    c.comp_mut(m).unwrap().volt = 1;
    c.wire((21, 4), (21, 34));
    // Bridges around the left of the motor down to W2 (x12), U2 (x15), V2 (x18).
    path(&mut c, &[(12, 32), (8, 32), (8, 47), (12, 47), (12, 45)]);
    path(&mut c, &[(15, 31), (7, 31), (7, 48), (15, 48), (15, 45)]);
    path(&mut c, &[(18, 30), (6, 30), (6, 49), (18, 49), (18, 45)]);
    start_stop(&mut c, 45, 4, "KM1");
    fit_left(&mut c);
    c
}

/// 5: Reversing with double interlock: each start button also opens the other
/// direction's branch (multi-contact buttons) besides the contactor NC contacts.
fn reversing_double() -> Circuit {
    let mut c = reversing();
    c.comps.retain(|k| k.pos.x < 40);
    c.wires.retain(|w| w.a.x < 40 && w.b.x < 40);
    c.add(TermL1, (45, 4).into(), 0, "");
    c.add(Fuse, (45, 6).into(), 0, "F1");
    c.wire((45, 4), (45, 6));
    c.wire((45, 12), (45, 14));
    c.add(PushNc, (45, 14).into(), 0, "S0");
    c.wire((45, 20), (45, 22));
    c.wire((45, 22), (68, 22));
    for (x, start, own, other) in [(45, "S1", "KM1", "KM2"), (62, "S2", "KM2", "KM1")] {
        let opp = if start == "S1" { "S2" } else { "S1" };
        c.wire((x, 22), (x, 24));
        c.add(PushNo, (x, 24).into(), 0, start);
        path(&mut c, &[(x + 6, 22), (x + 6, 24)]);
        c.add(ContactNo, (x + 6, 24).into(), 0, own);
        path(&mut c, &[(x + 6, 30), (x + 6, 32), (x, 32)]);
        c.wire((x, 30), (x, 34));
        // NC contact of the opposite button (same button, second contact).
        let nc = c.add(PushNc, (x, 34).into(), 0, opp);
        c.comp_mut(nc).unwrap().num = 1;
        c.wire((x, 40), (x, 42));
        c.add(ContactNc, (x, 42).into(), 0, other);
        c.wire((x, 48), (x, 50));
        c.add(Coil, (x, 50).into(), 0, own);
        c.wire((x, 56), (x, 60));
    }
    c.wire((45, 60), (62, 60));
    c.wire((45, 60), (45, 63));
    c.add(TermN, (45, 63).into(), 2, "");
    fit_left(&mut c);
    c
}

/// 6: M1 starts with S1; after KT1's delay M2 starts by itself (cascade).
fn cascade() -> Circuit {
    let mut c = Circuit::default();
    for (x, q, km, m) in [(9, "Q1", "KM1", "M1"), (28, "Q2", "KM2", "M2")] {
        supply3(&mut c, x, 4, Some(x + 9), 0);
        c.add(MotorBreaker3, (x, 8).into(), 0, q);
        col3(&mut c, x, 4, 8);
        c.add(Main3, (x, 18).into(), 0, km);
        col3(&mut c, x, 14, 18);
        col3(&mut c, x, 24, 30);
        c.add(Motor3, (x, 30).into(), 0, m);
        c.wire((x + 9, 4), (x + 9, 30));
    }
    let x = 50;
    c.add(TermL1, (x, 4).into(), 0, "");
    c.add(Fuse, (x, 6).into(), 0, "F1");
    c.wire((x, 4), (x, 6));
    c.wire((x, 12), (x, 14));
    c.add(PushNc, (x, 14).into(), 0, "S0");
    c.wire((x, 20), (x, 23));
    c.add(PushNo, (x, 23).into(), 0, "S1");
    path(&mut c, &[(x, 21), (x + 6, 21), (x + 6, 23)]);
    c.add(ContactNo, (x + 6, 23).into(), 0, "KM1");
    path(&mut c, &[(x + 6, 29), (x + 6, 31), (x, 31)]);
    c.wire((x, 29), (x, 33));
    c.wire((x, 33), (x + 22, 33));
    c.wire((x, 33), (x, 35));
    c.add(Coil, (x, 35).into(), 0, "KM1");
    c.wire((x, 41), (x, 54));
    c.wire((x + 11, 33), (x + 11, 35));
    let kt = c.add(TimerOn, (x + 11, 35).into(), 0, "KT1");
    c.comp_mut(kt).unwrap().preset = 5.0;
    c.wire((x + 11, 41), (x + 11, 54));
    c.wire((x + 22, 33), (x + 22, 35));
    c.add(ContactNo, (x + 22, 35).into(), 0, "KT1");
    c.wire((x + 22, 41), (x + 22, 44));
    c.add(Coil, (x + 22, 44).into(), 0, "KM2");
    c.wire((x + 22, 50), (x + 22, 54));
    c.wire((x, 54), (x + 22, 54));
    c.wire((x, 54), (x, 57));
    c.add(TermN, (x, 57).into(), 2, "");
    fit_left(&mut c);
    c
}

// ---------- single-phase ----------

/// 8: Single-phase motor with start winding Z1-Z2 in parallel with the main one.
fn single_direct() -> Circuit {
    let mut c = Circuit::default();
    c.add(TermL1, (10, 4).into(), 0, "");
    c.add(TermN, (13, 4).into(), 0, "");
    c.add(Breaker2, (10, 7).into(), 0, "Q1");
    c.wire((10, 4), (10, 7));
    c.wire((13, 4), (13, 7));
    c.add(Main3, (10, 17).into(), 0, "KM1");
    c.wire((10, 13), (10, 17));
    c.wire((13, 13), (13, 17));
    let m = c.add(Motor1S, (10, 30).into(), 0, "M1");
    c.comp_mut(m).unwrap().volt = 0;
    // L1 → U1 and Z1; N → U2 and Z2.
    path(&mut c, &[(10, 23), (10, 30)]);
    path(&mut c, &[(10, 27), (14, 27), (14, 30)]);
    path(&mut c, &[(13, 23), (13, 25), (18, 25), (18, 41), (10, 41), (10, 38)]);
    path(&mut c, &[(14, 41), (14, 38)]);
    start_stop(&mut c, 45, 4, "KM1");
    fit_left(&mut c);
    c
}

/// 9: Single-phase reversing: KM1 / KM2 swap the start winding's ends.
fn single_reversing() -> Circuit {
    let mut c = reversing();
    c.comps.retain(|k| k.pos.x >= 40);
    c.wires.retain(|w| w.a.x >= 40 && w.b.x >= 40);
    c.add(TermL1, (8, 4).into(), 0, "");
    c.add(TermN, (11, 4).into(), 0, "");
    c.add(Breaker2, (8, 7).into(), 0, "Q1");
    c.wire((8, 4), (8, 7));
    c.wire((11, 4), (11, 7));
    // L1 bus (y 15) and N bus (y 16).
    path(&mut c, &[(8, 13), (8, 15), (33, 15)]);
    path(&mut c, &[(11, 13), (11, 16), (36, 16)]);
    // KM1: L1→U1, L1→Z1, N→Z2.  KM2: L1→U1, N→Z1, L1→Z2.
    c.add(Main3, (14, 19).into(), 0, "KM1");
    c.add(Main3, (26, 19).into(), 0, "KM2");
    c.wire((14, 15), (14, 19));
    c.wire((17, 15), (17, 19));
    c.wire((20, 16), (20, 19));
    c.wire((26, 15), (26, 19));
    c.wire((29, 16), (29, 19));
    c.wire((32, 15), (32, 19));
    path(&mut c, &[(14, 25), (14, 27), (26, 27), (26, 25)]);
    path(&mut c, &[(17, 25), (17, 29), (29, 29), (29, 25)]);
    path(&mut c, &[(20, 25), (20, 31), (32, 31), (32, 25)]);
    let m = c.add(Motor1S, (14, 36).into(), 0, "M1");
    c.comp_mut(m).unwrap().volt = 0;
    c.wire((14, 27), (14, 36));
    c.wire((18, 29), (18, 36));
    path(&mut c, &[(23, 31), (23, 46), (18, 46), (18, 44)]);
    path(&mut c, &[(36, 16), (36, 48), (14, 48), (14, 44)]);
    fit_left(&mut c);
    c
}

// ---------- special motors ----------

/// Two-speed control: S1 low (KM1), S2 high (KM2 [+ KM3]), interlocked; stop with S0
/// before changing speed. Coils of `high` (KM2 and optional KM3) side by side.
fn two_speed_control(c: &mut Circuit, x: i32, with_km3: bool) {
    c.add(TermL1, (x, 4).into(), 0, "");
    c.add(Fuse, (x, 6).into(), 0, "F1");
    c.wire((x, 4), (x, 6));
    c.wire((x, 12), (x, 14));
    c.add(PushNc, (x, 14).into(), 0, "S0");
    c.wire((x, 20), (x, 22));
    c.wire((x, 22), (x + 20, 22));
    for (bx, s, own, other) in [(x, "S1", "KM1", "KM2"), (x + 14, "S2", "KM2", "KM1")] {
        c.wire((bx, 22), (bx, 24));
        c.add(PushNo, (bx, 24).into(), 0, s);
        path(c, &[(bx + 6, 22), (bx + 6, 24)]);
        c.add(ContactNo, (bx + 6, 24).into(), 0, own);
        path(c, &[(bx + 6, 30), (bx + 6, 32), (bx, 32)]);
        c.wire((bx, 30), (bx, 34));
        c.add(ContactNc, (bx, 34).into(), 0, other);
        c.wire((bx, 40), (bx, 42));
        c.add(Coil, (bx, 42).into(), 0, own);
        c.wire((bx, 48), (bx, 56));
    }
    if with_km3 {
        path(c, &[(x + 14, 41), (x + 24, 41), (x + 24, 42)]);
        c.add(Coil, (x + 24, 42).into(), 0, "KM3");
        c.wire((x + 24, 48), (x + 24, 56));
        c.wire((x, 56), (x + 24, 56));
    } else {
        c.wire((x, 56), (x + 14, 56));
    }
    c.wire((x, 56), (x, 59));
    c.add(TermN, (x, 59).into(), 2, "");
}

/// 10: Dahlander: low speed KM1 (Δ on 1U 1V 1W), high speed KM2 (2U 2V 2W) with
/// KM3 tying 1U-1V-1W (YY).
fn dahlander() -> Circuit {
    let mut c = Circuit::default();
    supply3(&mut c, 8, 4, Some(17), 0);
    c.add(MotorBreaker3, (8, 7).into(), 0, "Q1");
    col3(&mut c, 8, 4, 7);
    c.add(Main3, (8, 18).into(), 0, "KM1");
    col3(&mut c, 8, 13, 18);
    c.add(Main3, (26, 19).into(), 0, "KM2");
    path(&mut c, &[(8, 14), (26, 14), (26, 19)]);
    path(&mut c, &[(11, 15), (29, 15), (29, 19)]);
    path(&mut c, &[(14, 16), (32, 16), (32, 19)]);
    let m = c.add(MotorDahlander, (8, 32).into(), 0, "M1");
    c.comp_mut(m).unwrap().volt = 0;
    col3(&mut c, 8, 24, 32);
    c.wire((17, 4), (17, 32));
    path(&mut c, &[(26, 25), (26, 46), (8, 46), (8, 43)]);
    path(&mut c, &[(29, 25), (29, 47), (11, 47), (11, 43)]);
    path(&mut c, &[(32, 25), (32, 48), (14, 48), (14, 43)]);
    // KM3: star bridge on 1U 1V 1W for high speed.
    c.add(Main3, (38, 30).into(), 0, "KM3");
    path(&mut c, &[(8, 26), (38, 26), (38, 30)]);
    path(&mut c, &[(11, 27), (41, 27), (41, 30)]);
    path(&mut c, &[(14, 28), (44, 28), (44, 30)]);
    path(&mut c, &[(38, 36), (41, 36), (44, 36)]);
    two_speed_control(&mut c, 52, true);
    fit_left(&mut c);
    c
}

/// 15: Two separate windings: KM1 feeds the low-speed winding, KM2 the high-speed one.
fn two_winding() -> Circuit {
    let mut c = Circuit::default();
    supply3(&mut c, 8, 4, Some(17), 0);
    c.add(MotorBreaker3, (8, 7).into(), 0, "Q1");
    col3(&mut c, 8, 4, 7);
    c.add(Main3, (8, 18).into(), 0, "KM1");
    col3(&mut c, 8, 13, 18);
    c.add(Main3, (26, 19).into(), 0, "KM2");
    path(&mut c, &[(8, 14), (26, 14), (26, 19)]);
    path(&mut c, &[(11, 15), (29, 15), (29, 19)]);
    path(&mut c, &[(14, 16), (32, 16), (32, 19)]);
    let m = c.add(Motor2W, (8, 32).into(), 0, "M1");
    c.comp_mut(m).unwrap().volt = 0;
    col3(&mut c, 8, 24, 32);
    c.wire((17, 4), (17, 32));
    path(&mut c, &[(26, 25), (26, 46), (8, 46), (8, 43)]);
    path(&mut c, &[(29, 25), (29, 47), (11, 47), (11, 43)]);
    path(&mut c, &[(32, 25), (32, 48), (14, 48), (14, 43)]);
    two_speed_control(&mut c, 48, false);
    fit_left(&mut c);
    c
}

/// 12-terminal motor fed by KM1 with the given terminal links.
/// Motor at (8,32): top U1 8, V1 11, W1 14, U5 17, V5 20, W5 23 (y 32);
/// bottom U2 8, V2 11, W2 14, U6 17, V6 20, W6 23 (y 44).
fn m12(sys: u8, links: fn(&mut Circuit)) -> Circuit {
    let mut c = Circuit::default();
    supply3(&mut c, 8, 4, Some(26), sys);
    c.add(MotorBreaker3, (8, 8).into(), 0, "Q1");
    col3(&mut c, 8, 4, 8);
    c.add(Main3, (8, 18).into(), 0, "KM1");
    col3(&mut c, 8, 14, 18);
    col3(&mut c, 8, 24, 32);
    c.add(Motor12, (8, 32).into(), 0, "M1");
    c.wire((26, 4), (26, 32));
    links(&mut c);
    start_stop(&mut c, 50, 4, "KM1");
    fit_left(&mut c);
    c
}

fn m12_parallel(c: &mut Circuit) {
    // U1-U5, V1-V5, W1-W5 above the motor; U2-U6, V2-V6, W2-W6 below.
    path(c, &[(8, 28), (17, 28), (17, 32)]);
    path(c, &[(11, 29), (20, 29), (20, 32)]);
    path(c, &[(14, 30), (23, 30), (23, 32)]);
    path(c, &[(8, 44), (8, 46), (17, 46), (17, 44)]);
    path(c, &[(11, 44), (11, 47), (20, 47), (20, 44)]);
    path(c, &[(14, 44), (14, 48), (23, 48), (23, 44)]);
}

fn m12_series(c: &mut Circuit) {
    // U2-U5, V2-V5, W2-W5 around the right of the motor.
    path(c, &[(8, 44), (8, 46), (30, 46), (30, 28), (17, 28), (17, 32)]);
    path(c, &[(11, 44), (11, 47), (31, 47), (31, 29), (20, 29), (20, 32)]);
    path(c, &[(14, 44), (14, 48), (32, 48), (32, 30), (23, 30), (23, 32)]);
}

/// ΔΔ: parallel + U1-W2, V1-U2, W1-V2 (around the left).
fn m12_dd() -> Circuit {
    m12(1, |c| {
        m12_parallel(c);
        path(c, &[(8, 27), (4, 27), (4, 52), (14, 52), (14, 48)]);
        path(c, &[(11, 31), (3, 31), (3, 54), (8, 54), (8, 46)]);
        path(c, &[(14, 26), (2, 26), (2, 56), (11, 56), (11, 47)]);
    })
}

/// YY: parallel + U2-V2-W2 tied.
fn m12_yy() -> Circuit {
    m12(0, |c| {
        m12_parallel(c);
        path(c, &[(8, 46), (8, 50), (11, 50), (14, 50)]);
        c.wire((11, 47), (11, 50));
        c.wire((14, 48), (14, 50));
    })
}

/// Δ series: series + U1-W6, V1-U6, W1-V6.
fn m12_d() -> Circuit {
    m12(2, |c| {
        m12_series(c);
        path(c, &[(8, 27), (4, 27), (4, 52), (23, 52), (23, 44)]);
        path(c, &[(11, 31), (3, 31), (3, 54), (17, 54), (17, 44)]);
        path(c, &[(14, 26), (2, 26), (2, 56), (20, 56), (20, 44)]);
    })
}

/// Y series: series + U6-V6-W6 tied.
fn m12_y() -> Circuit {
    m12(3, |c| {
        m12_series(c);
        path(c, &[(17, 44), (20, 44), (23, 44)]);
    })
}

/// 16: Motor with a DC brake: the rectifier is fed from the motor terminals, so the
/// brake releases while the motor is energised and brakes as soon as KM1 opens.
fn brake_motor() -> Circuit {
    let mut c = Circuit::default();
    let y = power_line(&mut c, 10, "KM1", 19);
    col3(&mut c, 10, y, 34);
    c.add(Motor3, (10, 34).into(), 0, "M1");
    c.wire((19, 4), (19, 34));
    let r = c.add(Rectifier, (26, 30).into(), 0, "U1");
    c.comp_mut(r).unwrap().volt = 0;
    path(&mut c, &[(10, 27), (26, 27), (26, 30)]);
    path(&mut c, &[(13, 28), (29, 28), (29, 30)]);
    let b = c.add(Brake, (26, 39).into(), 0, "YB1");
    c.comp_mut(b).unwrap().volt = 0;
    c.wire((26, 36), (26, 39));
    path(&mut c, &[(29, 36), (29, 47), (26, 47), (26, 45)]);
    start_stop(&mut c, 45, 4, "KM1");
    fit_left(&mut c);
    c
}
/// Motor started by a push button with a holding contact, motor breaker and
/// signal lamps (running / breaker tripped).
pub fn direct_start() -> Circuit {
    let mut c = Circuit::default();
    // Power circuit.
    c.add(TermL1, (10, 10).into(), 0, "");
    c.add(TermL2, (13, 10).into(), 0, "");
    c.add(TermL3, (16, 10).into(), 0, "");
    c.add(TermPE, (19, 10).into(), 0, "");
    c.add(MotorBreaker3, (10, 16).into(), 0, "Q1");
    c.add(Main3, (10, 30).into(), 0, "KM1");
    c.add(Motor3, (10, 44).into(), 0, "M1");
    for x in [10, 13, 16] {
        c.wire((x, 10), (x, 16));
        c.wire((x, 22), (x, 30));
        c.wire((x, 36), (x, 44));
    }
    c.wire((19, 10), (19, 44));
    shift_left_part(&mut c, 4);

    // Control circuit.
    c.add(TermL1, (45, 8).into(), 0, "");
    c.add(Breaker1, (45, 11).into(), 0, "Q2");
    c.wire((45, 8), (45, 11));
    c.wire((45, 17), (45, 20));
    c.wire((45, 20), (75, 20));
    c.add(ContactNo, (45, 22).into(), 0, "Q1");
    c.wire((45, 20), (45, 22));
    c.add(PushNc, (45, 30).into(), 0, "S1");
    c.wire((45, 28), (45, 30));
    c.add(PushNo, (45, 40).into(), 0, "S2");
    c.wire((45, 36), (45, 40));
    c.wire((45, 38), (52, 38));
    c.wire((52, 38), (52, 40));
    c.add(ContactNo, (52, 40).into(), 0, "KM1");
    c.wire((52, 46), (52, 48));
    c.wire((52, 48), (45, 48));
    c.wire((45, 46), (45, 50));
    c.add(Coil, (45, 50).into(), 0, "KM1");
    c.wire((45, 56), (45, 60));
    c.add(TermN, (45, 60).into(), 2, "");
    c.wire((45, 60), (75, 60));
    // Running lamp.
    c.add(ContactNo, (65, 22).into(), 0, "KM1");
    c.wire((65, 20), (65, 22));
    c.wire((65, 28), (65, 50));
    c.add(Lamp, (65, 50).into(), 0, "H1");
    c.wire((65, 56), (65, 60));
    // Breaker tripped lamp.
    c.add(ContactNc, (75, 22).into(), 0, "Q1");
    c.wire((75, 20), (75, 22));
    c.wire((75, 28), (75, 50));
    let h2 = c.add(Lamp, (75, 50).into(), 0, "H2");
    c.comp_mut(h2).unwrap().color = 2;
    c.wire((75, 56), (75, 60));
    c
}

/// Forward / reverse with electrical interlocking.
pub fn reversing() -> Circuit {
    let mut c = Circuit::default();
    c.add(TermL1, (10, 8).into(), 0, "");
    c.add(TermL2, (13, 8).into(), 0, "");
    c.add(TermL3, (16, 8).into(), 0, "");
    c.add(TermPE, (19, 8).into(), 0, "");
    c.add(MotorBreaker3, (10, 12).into(), 0, "Q1");
    for x in [10, 13, 16] {
        c.wire((x, 8), (x, 12));
        c.wire((x, 18), (x, 26));
        c.wire((x, 32), (x, 42));
    }
    c.add(Main3, (10, 26).into(), 0, "KM1");
    c.add(Main3, (24, 26).into(), 0, "KM2");
    c.wire((10, 22), (24, 22));
    c.wire((24, 22), (24, 26));
    c.wire((13, 21), (27, 21));
    c.wire((27, 21), (27, 26));
    c.wire((16, 20), (30, 20));
    c.wire((30, 20), (30, 26));
    // KM2 swaps L1 and L3.
    c.wire((24, 32), (24, 38));
    c.wire((24, 38), (16, 38));
    c.wire((27, 32), (27, 37));
    c.wire((27, 37), (13, 37));
    c.wire((30, 32), (30, 36));
    c.wire((30, 36), (10, 36));
    c.add(Motor3, (10, 42).into(), 0, "M1");
    c.wire((19, 8), (19, 42));
    shift_left_part(&mut c, 4);

    c.add(TermL1, (45, 6).into(), 0, "");
    c.wire((45, 6), (45, 9));
    c.add(Fuse, (45, 9).into(), 0, "F1");
    c.wire((45, 15), (45, 18));
    c.add(PushNc, (45, 18).into(), 0, "S0");
    c.wire((45, 24), (45, 28));
    c.wire((45, 28), (67, 28));
    // Forward.
    c.wire((45, 28), (45, 30));
    c.add(PushNo, (45, 30).into(), 0, "S1");
    c.wire((52, 28), (52, 30));
    c.add(ContactNo, (52, 30).into(), 0, "KM1");
    c.wire((52, 36), (52, 38));
    c.wire((52, 38), (45, 38));
    c.wire((45, 36), (45, 40));
    c.add(ContactNc, (45, 40).into(), 0, "KM2");
    c.wire((45, 46), (45, 48));
    c.add(Coil, (45, 48).into(), 0, "KM1");
    c.wire((45, 54), (45, 58));
    // Reverse.
    c.wire((60, 28), (60, 30));
    c.add(PushNo, (60, 30).into(), 0, "S2");
    c.wire((67, 28), (67, 30));
    c.add(ContactNo, (67, 30).into(), 0, "KM2");
    c.wire((67, 36), (67, 38));
    c.wire((67, 38), (60, 38));
    c.wire((60, 36), (60, 40));
    c.add(ContactNc, (60, 40).into(), 0, "KM1");
    c.wire((60, 46), (60, 48));
    c.add(Coil, (60, 48).into(), 0, "KM2");
    c.wire((60, 54), (60, 58));
    c.wire((45, 58), (60, 58));
    c.wire((45, 58), (45, 62));
    c.add(TermN, (45, 62).into(), 2, "");
    c
}

/// Move everything drawn so far to the right (keeps tags clear of the frame).
fn shift_left_part(c: &mut Circuit, dx: i32) {
    for comp in c.comps.iter_mut() {
        comp.pos.x += dx;
    }
    for w in c.wires.iter_mut() {
        w.a.x += dx;
        w.b.x += dx;
    }
}

/// A lamp that lights a few seconds after the start button is pressed.
pub fn timed_lamp() -> Circuit {
    let mut c = Circuit::default();
    c.add(TermL1, (20, 6).into(), 0, "");
    c.wire((20, 6), (20, 10));
    c.wire((20, 10), (50, 10));
    c.add(PushNc, (20, 12).into(), 0, "S0");
    c.wire((20, 10), (20, 12));
    c.wire((20, 18), (20, 22));
    c.wire((20, 20), (27, 20));
    c.wire((27, 20), (27, 22));
    c.add(PushNo, (20, 22).into(), 0, "S1");
    c.add(ContactNo, (27, 22).into(), 0, "KM1");
    c.wire((27, 28), (27, 30));
    c.wire((27, 30), (20, 30));
    c.wire((20, 28), (20, 34));
    c.add(Coil, (20, 34).into(), 0, "KM1");
    c.wire((20, 32), (35, 32));
    c.wire((35, 32), (35, 34));
    let kt = c.add(TimerOn, (35, 34).into(), 0, "KT1");
    c.comp_mut(kt).unwrap().preset = 5.0;
    c.wire((20, 40), (20, 44));
    c.wire((35, 40), (35, 44));
    c.wire((20, 44), (50, 44));
    c.wire((20, 44), (20, 48));
    c.add(TermN, (20, 48).into(), 2, "");
    c.wire((50, 10), (50, 22));
    c.add(ContactNo, (50, 22).into(), 0, "KT1");
    c.wire((50, 28), (50, 34));
    let h = c.add(Lamp, (50, 34).into(), 0, "H1");
    c.comp_mut(h).unwrap().color = 2;
    c.wire((50, 40), (50, 44));
    c
}

/// Voltage demo: a 6-terminal motor closed in star on 220/380 V, and three
/// lamps of different ratings on a 127/220 V system (normal, dim, burnt).
pub fn voltages() -> Circuit {
    let mut c = Circuit::default();
    // Power: 220/380 V, motor Δ220/Y380 V closed in star (right for 380 V).
    c.add(TermL1, (10, 6).into(), 0, "");
    c.add(TermL2, (13, 6).into(), 0, "");
    c.add(TermL3, (16, 6).into(), 0, "");
    c.add(TermPE, (19, 6).into(), 0, "");
    c.add(MotorBreaker3, (10, 10).into(), 0, "Q1");
    c.add(Main3, (10, 20).into(), 0, "KM1");
    c.add(Motor6X, (10, 32).into(), 0, "M1");
    for x in [10, 13, 16] {
        c.wire((x, 6), (x, 10));
        c.wire((x, 16), (x, 20));
        c.wire((x, 26), (x, 32));
    }
    c.wire((19, 6), (19, 32));
    c.wire((10, 43), (13, 43));
    c.wire((13, 43), (16, 43));

    // Control: switch S1 turns KM1 on (220 V coil between L1 and N).
    c.add(TermL1, (40, 6).into(), 0, "");
    c.add(Switch, (40, 9).into(), 0, "S1");
    c.wire((40, 6), (40, 9));
    c.wire((40, 15), (40, 18));
    c.add(Coil, (40, 18).into(), 0, "KM1");
    c.wire((40, 24), (40, 28));
    c.add(TermN, (40, 28).into(), 2, "");

    // Lamps on a 127/220 V system: 127 V (normal), 220 V (dim), 24 V (burns).
    let l = c.add(TermL1, (52, 6).into(), 0, "");
    c.comp_mut(l).unwrap().volt = 1;
    c.wire((52, 6), (52, 10));
    c.wire((52, 10), (80, 10));
    for (x, tag, rating, color) in [(52, "H1", 1u8, 0u8), (66, "H2", 0, 2), (80, "H3", 3, 1)] {
        c.wire((x, 10), (x, 14));
        let h = c.add(Lamp, (x, 14).into(), 0, tag);
        let hc = c.comp_mut(h).unwrap();
        hc.volt = rating;
        hc.color = color;
        c.wire((x, 20), (x, 24));
    }
    c.wire((52, 24), (80, 24));
    c.wire((52, 24), (52, 28));
    c.add(TermN, (52, 28).into(), 2, "");
    shift_down(&mut c, 5);
    c
}
/// Move the whole drawing down (keeps it clear of the sheet header).
fn shift_down(c: &mut Circuit, dy: i32) {
    for comp in c.comps.iter_mut() {
        comp.pos.y += dy;
    }
    for w in c.wires.iter_mut() {
        w.a.y += dy;
        w.b.y += dy;
    }
}
/// Timed star-delta start: KM1 (line), KM2 (star) and KM3 (delta) with an
/// on-delay timer KT1 and electrical interlocks. The motor (Δ380 / Y660 V on a
/// 220/380 V system) starts in star with reduced voltage, then KT1 switches it to delta.
pub fn star_delta() -> Circuit {
    let mut c = Circuit::default();
    // ---- Power circuit ----
    c.add(TermL1, (6, 4).into(), 0, "");
    c.add(TermL2, (9, 4).into(), 0, "");
    c.add(TermL3, (12, 4).into(), 0, "");
    c.add(TermPE, (15, 4).into(), 0, "");
    c.add(MotorBreaker3, (6, 7).into(), 0, "Q1");
    c.add(Main3, (6, 17).into(), 0, "KM1");
    let m = c.add(Motor6X, (6, 30).into(), 0, "M1");
    c.comp_mut(m).unwrap().volt = 1; // Δ380 / Y660 V
    for x in [6, 9, 12] {
        c.wire((x, 4), (x, 7));
        c.wire((x, 13), (x, 17));
        c.wire((x, 23), (x, 30));
    }
    c.wire((15, 4), (15, 30));
    // Delta contactor KM3, fed from the lines after Q1: L1→W2, L2→U2, L3→V2.
    c.add(Main3, (22, 19).into(), 0, "KM3");
    c.wire((6, 14), (22, 14));
    c.wire((22, 14), (22, 19));
    c.wire((9, 15), (25, 15));
    c.wire((25, 15), (25, 19));
    c.wire((12, 16), (28, 16));
    c.wire((28, 16), (28, 19));
    // Crossed-winding motor: bottom terminals W2 U2 V2 sit under U1 V1 W1, so the
    // delta links go straight down: L1→W2 (x 6), L2→U2 (x 9), L3→V2 (x 12).
    c.wire((28, 25), (28, 46));
    c.wire((28, 46), (12, 46));
    c.wire((12, 46), (12, 41));
    c.wire((25, 25), (25, 48));
    c.wire((25, 48), (9, 48));
    c.wire((9, 48), (9, 41));
    c.wire((22, 25), (22, 50));
    c.wire((22, 50), (6, 50));
    c.wire((6, 50), (6, 41));
    // Star contactor KM2 joins U2, V2 and W2.
    c.add(Main3, (6, 53).into(), 0, "KM2");
    c.wire((6, 50), (6, 53));
    c.wire((9, 48), (9, 53));
    c.wire((12, 46), (12, 53));
    c.wire((6, 59), (9, 59));
    c.wire((9, 59), (12, 59));

    // ---- Control circuit (220 V) ----
    c.add(TermL1, (40, 4).into(), 0, "");
    c.add(Breaker1, (40, 6).into(), 0, "Q2");
    c.wire((40, 4), (40, 6));
    c.wire((40, 12), (40, 14));
    c.add(PushNc, (40, 14).into(), 0, "S0");
    c.wire((40, 20), (40, 23));
    c.add(PushNo, (40, 23).into(), 0, "S1");
    // Holding contact KM1 across S1.
    c.wire((40, 21), (46, 21));
    c.wire((46, 21), (46, 23));
    c.add(ContactNo, (46, 23).into(), 0, "KM1");
    c.wire((46, 29), (46, 31));
    c.wire((46, 31), (40, 31));
    c.wire((40, 29), (40, 33));
    c.wire((40, 33), (72, 33));
    // KM1 and KT1 coils.
    c.wire((40, 33), (40, 35));
    c.add(Coil, (40, 35).into(), 0, "KM1");
    c.wire((40, 41), (40, 62));
    c.wire((52, 33), (52, 35));
    let kt = c.add(TimerOn, (52, 35).into(), 0, "KT1");
    c.comp_mut(kt).unwrap().preset = 5.0;
    c.wire((52, 41), (52, 62));
    // Star: KT1 (timed opening) → KM3 NC interlock → KM2 coil.
    c.wire((61, 33), (61, 35));
    let a = c.add(ContactNc, (61, 35).into(), 0, "KT1");
    let _ = a;
    c.wire((61, 41), (61, 44));
    c.add(ContactNc, (61, 44).into(), 0, "KM3");
    c.wire((61, 50), (61, 53));
    c.add(Coil, (61, 53).into(), 0, "KM2");
    c.wire((61, 59), (61, 62));
    // Delta: KT1 (timed closing) → KM2 NC interlock → KM3 coil.
    c.wire((72, 33), (72, 35));
    let b = c.add(ContactNo, (72, 35).into(), 0, "KT1");
    let _ = b;
    c.wire((72, 41), (72, 44));
    c.add(ContactNc, (72, 44).into(), 0, "KM2");
    c.wire((72, 50), (72, 53));
    c.add(Coil, (72, 53).into(), 0, "KM3");
    c.wire((72, 59), (72, 62));
    c.wire((40, 62), (72, 62));
    c.wire((40, 62), (40, 64));
    c.add(TermN, (40, 64).into(), 2, "");
    shift_left_part(&mut c, 5);
    shift_down(&mut c, 3);
    c
}

/// Correction exercises: the examples with one subtle fault each that stops
/// them working. (name, what it should do, circuit)
pub fn corrections() -> Vec<(&'static str, &'static str, fn() -> Circuit)> {
    vec![
        (
            t("Correção 1 – Partida direta com sinalização", "Correction 1 – Direct start with indicator lamps"),
            t(
                "apertar S2 liga o motor e ele continua ligado ao soltar; S1 desliga.",
                "pressing S2 starts the motor and it keeps running when released; S1 stops it.",
            ),
            fault_seal as fn() -> Circuit,
        ),
        (
            t("Correção 2 – Partida direta", "Correction 2 – Direct start"),
            t("com KM1 ligado, o motor M1 deve girar.", "with KM1 on, motor M1 must turn."),
            fault_main as fn() -> Circuit,
        ),
        (
            t("Correção 3 – Reversão", "Correction 3 – Reversing"),
            t(
                "S1 gira o motor num sentido e S2 no sentido contrário.",
                "S1 turns the motor one way and S2 the other way.",
            ),
            fault_reverse as fn() -> Circuit,
        ),
        (
            t("Correção 4 – Lâmpada temporizada", "Correction 4 – Timed lamp"),
            t(
                "a lâmpada H1 acende 5 s depois de apertar S1.",
                "lamp H1 lights 5 s after pressing S1.",
            ),
            fault_timer as fn() -> Circuit,
        ),
        (
            t("Correção 5 – Estrela-triângulo", "Correction 5 – Star-delta"),
            t(
                "o motor parte em estrela (KM2) e após 5 s passa para triângulo (KM3).",
                "the motor starts in star (KM2) and after 5 s changes to delta (KM3).",
            ),
            fault_star_delta as fn() -> Circuit,
        ),
    ]
}

/// Holding contact tagged KM2 instead of KM1: the motor only runs while S2 is held.
fn fault_seal() -> Circuit {
    let mut c = direct_start();
    if let Some(k) = c.comps.iter_mut().find(|k| k.kind == ContactNo && k.tag == "KM1" && k.pos.y == 40) {
        k.tag = "KM2".into();
    }
    c
}

/// Main contacts follow "KM" (no such coil): the coil works but the motor never turns.
fn fault_main() -> Circuit {
    let mut c = direct_start();
    if let Some(k) = c.comps.iter_mut().find(|k| k.kind == Main3) {
        k.tag = "KM".into();
    }
    c
}

/// KM2's supply taps are crossed too, so the phase swap cancels out:
/// both contactors turn the motor the same way.
fn fault_reverse() -> Circuit {
    let mut c = reversing();
    let s = |x: i32, y: i32| P::new(x + 4, y);
    let drop = [
        (s(10, 22), s(24, 22)),
        (s(24, 22), s(24, 26)),
        (s(16, 20), s(30, 20)),
        (s(30, 20), s(30, 26)),
    ];
    c.wires.retain(|w| !drop.iter().any(|(a, b)| (w.a == *a && w.b == *b) || (w.a == *b && w.b == *a)));
    for (a, b) in [
        (s(10, 22), s(30, 22)),
        (s(30, 22), s(30, 26)),
        (s(16, 20), s(24, 20)),
        (s(24, 20), s(24, 26)),
    ] {
        c.wires.push(Wire { a, b });
    }
    c
}

/// The timer contact is NC instead of NO: the lamp lights at once and goes out after 5 s.
fn fault_timer() -> Circuit {
    let mut c = timed_lamp();
    if let Some(k) = c.comps.iter_mut().find(|k| k.kind == ContactNo && k.tag == "KT1") {
        k.kind = ContactNc;
    }
    c
}

/// KT1's contacts are swapped: it starts in delta and drops to star after 5 s.
fn fault_star_delta() -> Circuit {
    let mut c = star_delta();
    for k in c.comps.iter_mut().filter(|k| k.tag == "KT1" && k.kind.is_aux_contact()) {
        k.kind = if k.kind == Kind::ContactNo { ContactNc } else { ContactNo };
    }
    c
}