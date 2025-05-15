use std::{
    env, fs,
    io::{self, Stdout, Write},
    thread::sleep,
    time::Duration,
};

use chip8_core::{C8Emulator, SCREEN_HEIGHT, SCREEN_WIDTH};
use termion::{
    color, cursor,
    input::TermRead,
    raw::{IntoRawMode, RawTerminal},
    screen::{AlternateScreen, IntoAlternateScreen},
};

const TICK_PER_FRAME: u8 = 10;

fn main() -> io::Result<()> {
    let mut ch8 = C8Emulator::new();

    let mut args = env::args();

    args.next(); // just ignore the first item, it is the program name.

    let file_path = match args.next() {
        Some(arg) => arg,
        None => panic!("Didn't get a file path"),
    };

    let rom = fs::read(file_path).expect("Error reading rom");

    // Set up terminal
    let mut stdout = io::stdout().into_raw_mode()?.into_alternate_screen()?;

    ch8.load(&rom);

    let mut keys = termion::async_stdin().keys();
    let mut last_key: Option<u8> = None;

    loop {
        if let Some(key) = keys.next() {
            let pressed_key = match key.unwrap() {
                termion::event::Key::Char(chr) => map_ch8_key(chr),
                termion::event::Key::Esc => break,
                _ => None,
            };

            if let Some(key) = last_key {
                ch8.press_key(key as usize, false);
            };

            if let Some(key) = pressed_key {
                ch8.press_key(key as usize, true);
                last_key = Some(key);
            };
        }

        for _ in 0..TICK_PER_FRAME {
            ch8.cpu_cycle();
            sleep(Duration::from_millis(1));
        }
        ch8.frame_cycle();

        refresh_screen(&mut stdout, &ch8)?;
    }

    Ok(())
}

fn refresh_screen(
    stdout: &mut AlternateScreen<RawTerminal<Stdout>>,
    ch8: &C8Emulator,
) -> io::Result<()> {
    let screen = ch8.get_screen();

    write!(stdout, "{}", termion::clear::All)?;
    stdout.flush()?;

    for y in 0..SCREEN_HEIGHT {
        let line: String = screen[(y * SCREEN_WIDTH)..((y + 1) * SCREEN_WIDTH)]
            .iter()
            .map(|x| if *x { '*' } else { ' ' })
            .collect();

        write!(
            stdout,
            "{}{}{}",
            cursor::Goto(1, y as u16 + 1),
            color::Fg(color::Green),
            line
        )?;
    }

    stdout.flush()?;

    Ok(())
}

fn map_ch8_key(chr: char) -> Option<u8> {
    match chr {
        '1' => Some(0x1),
        '2' => Some(0x2),
        '3' => Some(0x3),
        '4' => Some(0xC),
        'q' => Some(0x4),
        'w' => Some(0x5),
        'e' => Some(0x6),
        'r' => Some(0xD),
        'a' => Some(0x7),
        's' => Some(0x8),
        'd' => Some(0x9),
        'f' => Some(0xE),
        'z' => Some(0xA),
        'x' => Some(0x0),
        'c' => Some(0xB),
        'v' => Some(0xF),
        _ => None,
    }
}
