use crate::json::J;

pub const COMPLETION_SOUND_OPTIONS: J = J::Arr(&[
    J::Obj(&[
        ("fileName", J::Str("ping.mp3")),
        ("label", J::Str("Ping")),
        ("value", J::Str("ping")),
    ]),
    J::Obj(&[
        ("fileName", J::Str("pingdouble.mp3")),
        ("label", J::Str("Ping Double")),
        ("value", J::Str("pingdouble")),
    ]),
    J::Obj(&[
        ("fileName", J::Str("glass.mp3")),
        ("label", J::Str("Glass")),
        ("value", J::Str("glass")),
    ]),
    J::Obj(&[
        ("fileName", J::Str("glimmer.mp3")),
        ("label", J::Str("Glimmer")),
        ("value", J::Str("glimmer")),
    ]),
    J::Obj(&[
        ("fileName", J::Str("shamisen.mp3")),
        ("label", J::Str("Shamisen")),
        ("value", J::Str("shamisen")),
    ]),
    J::Obj(&[
        ("fileName", J::Str("shamisenreverb.mp3")),
        ("label", J::Str("Shamisen Reverb")),
        ("value", J::Str("shamisenreverb")),
    ]),
    J::Obj(&[
        ("fileName", J::Str("arcade.mp3")),
        ("label", J::Str("Arcade")),
        ("value", J::Str("arcade")),
    ]),
    J::Obj(&[
        ("fileName", J::Str("arcadeboost.mp3")),
        ("label", J::Str("Arcade Boost")),
        ("value", J::Str("arcadeboost")),
    ]),
    J::Obj(&[
        ("fileName", J::Str("confirmation-001.mp3")),
        ("label", J::Str("Confirmation 001")),
        ("value", J::Str("confirmation-001")),
    ]),
    J::Obj(&[
        ("fileName", J::Str("confirmation-002.mp3")),
        ("label", J::Str("Confirmation 002")),
        ("value", J::Str("confirmation-002")),
    ]),
    J::Obj(&[
        ("fileName", J::Str("confirmation-003.mp3")),
        ("label", J::Str("Confirmation 003")),
        ("value", J::Str("confirmation-003")),
    ]),
    J::Obj(&[
        ("fileName", J::Str("confirmation-004.mp3")),
        ("label", J::Str("Confirmation 004")),
        ("value", J::Str("confirmation-004")),
    ]),
    J::Obj(&[
        ("fileName", J::Str("notification-pop.mp3")),
        ("label", J::Str("Notification Pop")),
        ("value", J::Str("notification-pop")),
    ]),
    J::Obj(&[
        ("fileName", J::Str("success-chime.mp3")),
        ("label", J::Str("Success Chime")),
        ("value", J::Str("success-chime")),
    ]),
    J::Obj(&[
        ("fileName", J::Str("high-up.mp3")),
        ("label", J::Str("High Up")),
        ("value", J::Str("high-up")),
    ]),
    J::Obj(&[
        ("fileName", J::Str("high-down.mp3")),
        ("label", J::Str("High Down")),
        ("value", J::Str("high-down")),
    ]),
    J::Obj(&[
        ("fileName", J::Str("low-three-tone.mp3")),
        ("label", J::Str("Low Three Tone")),
        ("value", J::Str("low-three-tone")),
    ]),
    J::Obj(&[
        ("fileName", J::Str("tone-1.mp3")),
        ("label", J::Str("Tone 1")),
        ("value", J::Str("tone-1")),
    ]),
    J::Obj(&[
        ("fileName", J::Str("three-tone-1.mp3")),
        ("label", J::Str("Three Tone 1")),
        ("value", J::Str("three-tone-1")),
    ]),
    J::Obj(&[
        ("fileName", J::Str("three-tone-2.mp3")),
        ("label", J::Str("Three Tone 2")),
        ("value", J::Str("three-tone-2")),
    ]),
    J::Obj(&[
        ("fileName", J::Str("two-tone-1.mp3")),
        ("label", J::Str("Two Tone 1")),
        ("value", J::Str("two-tone-1")),
    ]),
    J::Obj(&[
        ("fileName", J::Str("two-tone-2.mp3")),
        ("label", J::Str("Two Tone 2")),
        ("value", J::Str("two-tone-2")),
    ]),
    J::Obj(&[
        ("fileName", J::Str("power-up-5.mp3")),
        ("label", J::Str("Power Up 5")),
        ("value", J::Str("power-up-5")),
    ]),
    J::Obj(&[
        ("fileName", J::Str("power-up-6.mp3")),
        ("label", J::Str("Power Up 6")),
        ("value", J::Str("power-up-6")),
    ]),
    J::Obj(&[
        ("fileName", J::Str("power-up-8.mp3")),
        ("label", J::Str("Power Up 8")),
        ("value", J::Str("power-up-8")),
    ]),
    J::Obj(&[
        ("fileName", J::Str("coin-collect.mp3")),
        ("label", J::Str("Coin Collect")),
        ("value", J::Str("coin-collect")),
    ]),
    J::Obj(&[
        ("fileName", J::Str("phaser-up-5.mp3")),
        ("label", J::Str("Phaser Up 5")),
        ("value", J::Str("phaser-up-5")),
    ]),
    J::Obj(&[
        ("fileName", J::Str("zap-two-tone.mp3")),
        ("label", J::Str("Zap Two Tone")),
        ("value", J::Str("zap-two-tone")),
    ]),
    J::Obj(&[
        (
            "fileName",
            J::Str("voiceover-pack-male-mission-completed.mp3"),
        ),
        ("label", J::Str("Mission Completed (Male)")),
        ("value", J::Str("voiceover-pack-male-mission-completed")),
    ]),
    J::Obj(&[
        (
            "fileName",
            J::Str("voiceover-pack-female-mission-completed.mp3"),
        ),
        ("label", J::Str("Mission Completed (Female)")),
        ("value", J::Str("voiceover-pack-female-mission-completed")),
    ]),
    J::Obj(&[
        ("fileName", J::Str("voiceover-pack-male-you-win.mp3")),
        ("label", J::Str("You Win (Male)")),
        ("value", J::Str("voiceover-pack-male-you-win")),
    ]),
    J::Obj(&[
        (
            "fileName",
            J::Str("voiceover-pack-female-congratulations.mp3"),
        ),
        ("label", J::Str("Congratulations (Female)")),
        ("value", J::Str("voiceover-pack-female-congratulations")),
    ]),
    J::Obj(&[
        ("fileName", J::Str("flawless-victory.mp3")),
        ("label", J::Str("Flawless Victory")),
        ("value", J::Str("flawless-victory")),
    ]),
]);
