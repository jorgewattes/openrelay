//! Ready-made circuits shown in Arquivo → Exemplos.

use crate::circuit::{t, Circuit, Kind::*};

pub fn list() -> Vec<(&'static str, fn() -> Circuit)> {
    vec![
        (t("Partida direta com selo", "Direct-on-line start/stop"), direct_start as fn() -> Circuit),
        (t("Reversão de motor", "Forward / reverse motor"), reversing as fn() -> Circuit),
        (t("Lâmpada temporizada", "Timed lamp"), timed_lamp as fn() -> Circuit),
        (t("Partida estrela-triângulo temporizada", "Timed star-delta start"), star_delta as fn() -> Circuit),
        (t("Tensões: motor Y/Δ e lâmpadas", "Voltages: Y/Δ motor and lamps"), voltages as fn() -> Circuit),
    ]
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
    c.add(Breaker3, (10, 16).into(), 0, "Q1");
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
    c.add(Breaker3, (10, 12).into(), 0, "Q1");
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
    c.add(Breaker3, (10, 10).into(), 0, "Q1");
    c.add(Main3, (10, 20).into(), 0, "KM1");
    c.add(Motor6, (10, 32).into(), 0, "M1");
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
    c.add(Breaker3, (6, 7).into(), 0, "Q1");
    c.add(Main3, (6, 17).into(), 0, "KM1");
    let m = c.add(Motor6, (6, 30).into(), 0, "M1");
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
    c.wire((22, 25), (22, 46));
    c.wire((22, 46), (12, 46));
    c.wire((12, 46), (12, 41));
    c.wire((25, 25), (25, 48));
    c.wire((25, 48), (6, 48));
    c.wire((6, 48), (6, 41));
    c.wire((28, 25), (28, 50));
    c.wire((28, 50), (9, 50));
    c.wire((9, 50), (9, 41));
    // Star contactor KM2 joins U2, V2 and W2.
    c.add(Main3, (6, 53).into(), 0, "KM2");
    c.wire((6, 48), (6, 53));
    c.wire((9, 50), (9, 53));
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
    c.comp_mut(a).unwrap().num = 0;
    c.wire((61, 41), (61, 44));
    c.add(ContactNc, (61, 44).into(), 0, "KM3");
    c.wire((61, 50), (61, 53));
    c.add(Coil, (61, 53).into(), 0, "KM2");
    c.wire((61, 59), (61, 62));
    // Delta: KT1 (timed closing) → KM2 NC interlock → KM3 coil.
    c.wire((72, 33), (72, 35));
    let b = c.add(ContactNo, (72, 35).into(), 0, "KT1");
    c.comp_mut(b).unwrap().num = 0;
    c.wire((72, 41), (72, 44));
    c.add(ContactNc, (72, 44).into(), 0, "KM2");
    c.wire((72, 50), (72, 53));
    c.add(Coil, (72, 53).into(), 0, "KM3");
    c.wire((72, 59), (72, 62));
    c.wire((40, 62), (72, 62));
    c.wire((40, 62), (40, 64));
    c.add(TermN, (40, 64).into(), 2, "");
    shift_left_part(&mut c, 3);
    shift_down(&mut c, 3);
    c
}
