use project_x_terminal::{Engine, Error, Input, InputError, Key, KeyPhase, Modifiers};

fn row(engine: &Engine, row: usize) -> String {
    let frame = engine.frame();
    frame.cells[row * usize::from(frame.cols)..(row + 1) * usize::from(frame.cols)]
        .iter()
        .map(|cell| cell.text.as_str())
        .collect()
}

fn feed(engine: &mut Engine, bytes: &[u8]) -> Vec<u8> {
    bytes
        .chunks(512)
        .flat_map(|bytes| engine.feed(bytes).unwrap().replies)
        .collect()
}

fn key(key: Key) -> Input {
    Input::Key {
        key,
        modifiers: Modifiers::default(),
        phase: KeyPhase::Press,
    }
}

#[test]
fn geometry_and_operation_caps_refuse_before_mutating() {
    for (cols, rows) in [(0, 24), (80, 1), (161, 24), (80, 49)] {
        assert!(matches!(
            Engine::new(cols, rows),
            Err(Error::InvalidGeometry)
        ));
    }
    let mut engine = Engine::new(80, 24).unwrap();
    let before = engine.frame();
    assert_eq!(engine.feed(&[b'a'; 513]), Err(Error::FeedTooLarge));
    assert_eq!(engine.resize(161, 24), Err(Error::InvalidGeometry));
    assert_eq!(engine.frame(), before);
    assert_eq!(
        engine.encode(&Input::Text {
            text: "a".repeat(8193)
        }),
        Err(InputError::TooLarge)
    );
}

#[test]
fn cursor_erase_attributes_and_whole_sequence_validation() {
    let mut engine = Engine::new(10, 4).unwrap();
    feed(
        &mut engine,
        b"abcdefghij\x1b[2;3H\x1b[31;1mX\x1b[31;3mY\x1b[0mZ",
    );
    assert_eq!(row(&engine, 1), "  XYZ     ");
    let frame = engine.frame();
    assert_eq!(frame.cells[12].attrs.foreground, Some(1));
    assert!(frame.cells[13].attrs.bold);
    assert_eq!(frame.cells[14].attrs.foreground, None);
    feed(&mut engine, b"\x1b[2;4H\x1b[1K");
    assert_eq!(row(&engine, 1), "    Z     ");
    feed(&mut engine, b"\x1b[2J");
    assert!(engine.frame().cells.iter().all(|cell| cell.text == " "));
}

#[test]
fn wrap_scroll_margin_origin_and_tabs_are_owned_state() {
    let mut engine = Engine::new(10, 4).unwrap();
    feed(&mut engine, b"1234567890X");
    assert_eq!(row(&engine, 0), "1234567890");
    assert_eq!(row(&engine, 1), "X         ");
    feed(&mut engine, b"\x1b[2;3r\x1b[?6hA\x1b[2;1HB\nC");
    assert_eq!(row(&engine, 0), "1234567890");
    assert_eq!(row(&engine, 1), "B         ");
    assert_eq!(row(&engine, 2), " C        ");
    assert_eq!(feed(&mut engine, b"\x1b[6n"), b"\x1b[2;3R");
    feed(&mut engine, b"\x1b[?6l\x1b[1;1H\tT");
    assert_eq!(engine.frame().cells[8].text, "T");
}

#[test]
fn ordinary_save_and_1049_return_are_distinct() {
    let mut engine = Engine::new(10, 4).unwrap();
    feed(
        &mut engine,
        b"primary\x1b7\x1b[3;4H\x1b[?1049halt\x1b[?1049h!",
    );
    assert!(engine.frame().alternate);
    assert_eq!(row(&engine, 0), "alt!      ");
    feed(&mut engine, b"\x1b[?1049lX\x1b8Y");
    assert!(!engine.frame().alternate);
    assert_eq!(row(&engine, 0), "primaryY  ");
    assert_eq!(engine.frame().cells[23].text, "X");
    feed(&mut engine, b"\x1b[?1049h");
    assert_eq!(row(&engine, 0), "          ");
    feed(&mut engine, b"\x1b[?1049l");
    assert_eq!(row(&engine, 0), "primaryY  ");
}

#[test]
fn resize_preserves_both_intersections_and_does_not_reflow() {
    let mut engine = Engine::new(10, 4).unwrap();
    feed(&mut engine, b"0123456789\x1b[?1049hOTHER");
    engine.resize(4, 3).unwrap();
    assert_eq!(row(&engine, 0), "OTHE");
    feed(&mut engine, b"\x1b[?1049l");
    assert_eq!(row(&engine, 0), "0123");
    engine.resize(10, 4).unwrap();
    assert_eq!(row(&engine, 0), "0123      ");
}

#[test]
fn utf8_is_incremental_and_frames_never_contain_control_text() {
    let mut engine = Engine::new(20, 4).unwrap();
    feed(&mut engine, &[b'a', 0xc3]);
    assert_eq!(row(&engine, 0), "a                   ");
    feed(
        &mut engine,
        &[
            0xa9, 0xe2, 0x82, 0xac, 0xe0, 0x80, 27, b'[', b'2', b'C', 0x9b,
        ],
    );
    assert_eq!(&row(&engine, 0)[.."aé���".len()], "aé���");
    assert!(
        engine
            .frame()
            .cells
            .iter()
            .all(|cell| cell.text.chars().all(|ch| !ch.is_control()))
    );
    feed(&mut engine, &[0xc3]);
    engine.finish_output().unwrap();
    assert_eq!(engine.feed(b"x"), Err(Error::Closed));
    assert_eq!(engine.encode(&key(Key::Enter)), Err(InputError::Closed));
    let final_frame = engine.frame();
    engine.finish_output().unwrap();
    assert_eq!(engine.frame(), final_frame);
}

#[test]
fn strings_discard_controls_and_have_constant_retained_payload() {
    let mut engine = Engine::new(10, 4).unwrap();
    feed(&mut engine, b"a\x1b]52;c;secret\n\x1b[2J");
    for _ in 0..100 {
        feed(&mut engine, &[b'x'; 512]);
    }
    feed(&mut engine, b"\x1b\\b\x1bPignored\x07\n\x1b\\c");
    assert_eq!(row(&engine, 0), "abc       ");
    assert_eq!(engine.frame().cursor_row, 0);
}

#[test]
fn malformed_overflow_and_cancel_do_not_reinterpret_tail_as_text() {
    let mut engine = Engine::new(10, 4).unwrap();
    feed(&mut engine, b"\x1b[999999999999");
    feed(&mut engine, &[0xff; 512]);
    feed(&mut engine, b"CX\x1b[1:2JY\x1b[123\x18Z");
    assert_eq!(row(&engine, 0), "XYZ       ");
    assert_eq!(engine.frame().diagnostics.malformed_sequences, 2);
    feed(&mut engine, b"\x1b[");
    for _ in 0..3 {
        feed(&mut engine, &[0; 512]);
    }
    feed(&mut engine, b"2JQ");
    assert_eq!(row(&engine, 0), "XYZQ      ");
}

#[test]
fn replies_are_exact_ordered_proposals_without_reparsing() {
    let mut engine = Engine::new(10, 4).unwrap();
    assert_eq!(
        feed(
            &mut engine,
            b"\x1b[2;3H\x1b[5n\x1b[6n\x1b[?6n\x1b[?2004$p\x1b[?999$p\x1b[4$p\x1b[c"
        ),
        b"\x1b[0n\x1b[2;3R\x1b[?2;3R\x1b[?2004;2$y\x1b[?999;0$y\x1b[4;0$y"
    );
    assert!(engine.frame().cells.iter().all(|cell| cell.text == " "));
    let worst = b"\x1b[5n".repeat(128);
    let replies = feed(&mut engine, &worst);
    assert_eq!(replies.len(), 512);
}

#[test]
fn all_two_part_cuts_produce_identical_cells_modes_diagnostics_and_replies() {
    let data=b"hello\rX\n\xc3\xa9\x1b[31;1mR\x1b[0m\x1b[?1h\x1b[?2004h\x1b[2;3H\x1b[6n\x1b]ignore\x1b\\Z\x1b[?1049halt\x1b[?1049l";
    let mut whole = Engine::new(20, 4).unwrap();
    let expected = feed(&mut whole, data);
    let frame = whole.frame();
    for cut in 0..=data.len() {
        let mut split = Engine::new(20, 4).unwrap();
        let mut replies = feed(&mut split, &data[..cut]);
        replies.extend(feed(&mut split, &data[cut..]));
        let actual = split.frame();
        assert_eq!(replies, expected, "cut {cut}");
        assert_eq!(actual.cells, frame.cells, "cut {cut}");
        assert_eq!(actual.modes, frame.modes, "cut {cut}");
        assert_eq!(actual.diagnostics, frame.diagnostics, "cut {cut}");
        assert_eq!(
            (actual.cursor_row, actual.cursor_col),
            (frame.cursor_row, frame.cursor_col)
        );
    }
}

#[test]
fn aliases_actual_named_keys_repeats_and_releases_have_explicit_semantics() {
    let engine = Engine::new(80, 24).unwrap();
    for (ch, named, byte) in [
        ('i', Key::Tab, 9),
        ('m', Key::Enter, 13),
        ('[', Key::Escape, 27),
    ] {
        assert_eq!(
            engine
                .encode(&Input::Key {
                    key: Key::Character(ch),
                    modifiers: Modifiers {
                        ctrl: true,
                        ..Default::default()
                    },
                    phase: KeyPhase::Press
                })
                .unwrap(),
            vec![byte]
        );
        assert_eq!(engine.encode(&key(named)).unwrap(), vec![byte]);
    }
    assert_eq!(
        engine
            .encode(&Input::Key {
                key: Key::Character('C'),
                modifiers: Modifiers {
                    ctrl: true,
                    alt: true,
                    ..Default::default()
                },
                phase: KeyPhase::Repeat
            })
            .unwrap(),
        b"\x1b\x03"
    );
    assert_eq!(
        engine.encode(&Input::Key {
            key: Key::Left,
            modifiers: Modifiers::default(),
            phase: KeyPhase::Release
        }),
        Err(InputError::UnsupportedRelease)
    );
    assert_eq!(
        engine.encode(&Input::Key {
            key: Key::Character('x'),
            modifiers: Modifiers {
                shift: true,
                ..Default::default()
            },
            phase: KeyPhase::Press
        }),
        Err(InputError::UnsupportedModifiers)
    );
}

#[test]
fn arrows_keypad_and_modified_keys_follow_same_canonical_modes() {
    let mut engine = Engine::new(80, 24).unwrap();
    assert_eq!(engine.encode(&key(Key::Up)).unwrap(), b"\x1b[A");
    let initial = engine.frame().mode_revision;
    feed(&mut engine, b"echo");
    assert_eq!(engine.frame().mode_revision, initial);
    feed(&mut engine, b"\x1b[?1h\x1b=");
    assert_eq!(engine.encode(&key(Key::Up)).unwrap(), b"\x1bOA");
    assert_eq!(engine.encode(&key(Key::KeypadEnter)).unwrap(), b"\x1bOM");
    assert_eq!(
        engine
            .encode(&Input::Key {
                key: Key::Left,
                modifiers: Modifiers {
                    ctrl: true,
                    ..Default::default()
                },
                phase: KeyPhase::Press
            })
            .unwrap(),
        b"\x1b[1;5D"
    );
    feed(&mut engine, b"\x1b[?1l\x1b>");
    assert_eq!(engine.encode(&key(Key::Up)).unwrap(), b"\x1b[A");
    assert_eq!(engine.encode(&key(Key::KeypadEnter)).unwrap(), b"\r");
    let before = engine.frame().mode_revision;
    feed(&mut engine, b"\x1b[?1h\x1b[?1l");
    assert_eq!(engine.frame().mode_revision, before + 1);
}

#[test]
fn unsupported_enhancement_fences_input_until_reset_without_fake_encoding() {
    for negotiation in [b"\x1b[>1u".as_slice(), b"\x1b[>4;2m".as_slice()] {
        let mut engine = Engine::new(80, 24).unwrap();
        feed(&mut engine, negotiation);
        assert_eq!(engine.encode(&key(Key::Tab)), Err(InputError::UnknownModes));
        assert!(!engine.frame().modes.input_protocol_known);
        feed(&mut engine, b"\x1bc");
        assert_eq!(engine.encode(&key(Key::Tab)).unwrap(), b"\t");
    }
}

#[test]
fn text_paste_and_focus_refuse_hidden_controls_or_silent_unbracketed_paste() {
    let mut engine = Engine::new(80, 24).unwrap();
    assert_eq!(
        engine.encode(&Input::Text { text: "é".into() }).unwrap(),
        "é".as_bytes()
    );
    for text in ["a\n", "\x1b[2J", "🙂", "\u{ad}"] {
        assert_eq!(
            engine.encode(&Input::Text { text: text.into() }),
            Err(InputError::UnsupportedText)
        );
    }
    assert_eq!(
        engine.encode(&Input::Paste {
            text: "ls\n".into()
        }),
        Err(InputError::PasteNotEnabled)
    );
    feed(&mut engine, b"\x1b[?2004h\x1b[?1004h");
    assert_eq!(
        engine
            .encode(&Input::Paste {
                text: "a\r\nb\rc\t".into()
            })
            .unwrap(),
        b"\x1b[200~a\nb\nc\t\x1b[201~"
    );
    assert_eq!(
        engine.encode(&Input::Paste {
            text: "\x1b[201~".into()
        }),
        Err(InputError::UnsupportedText)
    );
    assert_eq!(
        engine.encode(&Input::Focus { focused: true }).unwrap(),
        b"\x1b[I"
    );
}

#[test]
fn wire_roundtrip_rejects_unrecognized_intent_fields() {
    let input = Input::Key {
        key: Key::Character('é'),
        modifiers: Modifiers::default(),
        phase: KeyPhase::Press,
    };
    assert_eq!(
        serde_json::from_str::<Input>(&serde_json::to_string(&input).unwrap()).unwrap(),
        input
    );
    assert!(
        serde_json::from_str::<Input>(r#"{"kind":"Text","text":"safe","raw":"secret"}"#).is_err()
    );
    let frame = Engine::new(80, 24).unwrap().frame();
    assert_eq!(
        serde_json::from_str::<project_x_terminal::Frame>(&serde_json::to_string(&frame).unwrap())
            .unwrap(),
        frame
    );
}

#[test]
fn deterministic_hostile_byte_corpus_is_partition_invariant_and_stays_bounded() {
    for seed in 1_u64..=64 {
        let mut state = seed;
        let bytes: Vec<u8> = (0..512)
            .map(|_| {
                state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
                (state >> 32) as u8
            })
            .collect();
        let mut whole = Engine::new(20, 4).unwrap();
        let expected_replies = whole.feed(&bytes).unwrap().replies;
        let expected = whole.frame();
        let mut split = Engine::new(20, 4).unwrap();
        let mut actual_replies = Vec::new();
        for chunk in bytes.chunks(seed as usize) {
            actual_replies.extend(split.feed(chunk).unwrap().replies);
        }
        let actual = split.frame();
        assert_eq!(actual_replies, expected_replies, "seed {seed}");
        assert_eq!(actual.cells, expected.cells, "seed {seed}");
        assert_eq!(actual.modes, expected.modes, "seed {seed}");
        assert_eq!(actual.diagnostics, expected.diagnostics, "seed {seed}");
        assert_eq!(actual.cells.len(), 80);
        assert!(
            actual
                .cells
                .iter()
                .all(|cell| cell.text.chars().count() == 1
                    && !cell.text.chars().next().unwrap().is_control())
        );
    }
}
