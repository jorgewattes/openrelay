//! Ready-made circuits shown in Arquivo → Exemplos.

use crate::circuit::{t, Circuit, Kind::*};

pub fn list() -> Vec<(&'static str, fn() -> Circuit)> {
    vec![
        (t("Partida direta com selo", "Direct-on-line start/stop"), direct_start as fn() -> Circuit),
        (t("Reversão de motor", "Forward / reverse motor"), reversing as fn() -> Circuit),
        (t("Lâmpada temporizada", "Timed lamp"), timed_lamp as fn() -> Circuit),
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
