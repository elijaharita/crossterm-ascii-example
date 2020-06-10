extern crate crossterm;

use crossterm::{cursor, style, terminal, ExecutableCommand, QueueableCommand};
use std::io::{stdin, stdout, Read, Write};
use std::sync::mpsc::channel;

// NOTES

// .queue() and .execute() commands are exposed by crossterm's QueueableCommand
// and ExecutableCommand traits. .queue() queues a command to be executed next
// time the output buffer is flushed, while .execute() immediately causes the
// output to flush
// .execute(...)? is effectively the same as .queue(...)?.flush()?
// queueing is useful for stacking terminal writes / commands for better
// performance

fn main() {
    // the terminal has to be prepared for effective operation
    setup_terminal();

    run_game().ok();

    // the terminal will not automatically return to its initial state after the
    // program exits, so we must make sure we undo each initialization step
    // manually
    cleanup_terminal();
}

fn setup_terminal() {
    // STEP 1
    // switch to the alternate terminal window
    // the alternate terminal window empties out the terminal's contents and
    // effectively acts as a new terminal until alternate window is disabled,
    // at which point the previous contents will be restored
    stdout().execute(terminal::EnterAlternateScreen).unwrap();

    // STEP 2
    // enable the terminal raw mode
    // this allows stdin to receive immediate keyboard input without waiting for
    // the user to press enter
    // this is useful for real time game controls
    // be aware that this function affects both terminal input and output
    terminal::enable_raw_mode().unwrap();

    // STEP 3
    // disable the cursor
    stdout().execute(cursor::Hide).unwrap();
}

fn cleanup_terminal() {
    // STEP 1
    // reenable the cursor
    stdout().execute(cursor::Show).unwrap();
    
    // STEP 2
    // disable raw mode
    terminal::disable_raw_mode().unwrap();

    // STEP 3
    // leave the alternate screen
    stdout().execute(terminal::LeaveAlternateScreen).unwrap();
}

#[derive(Debug, Clone)]
struct GameError;

impl From<std::io::Error> for GameError {
    fn from(_: std::io::Error) -> Self { GameError {} }
}

impl From<crossterm::ErrorKind> for GameError {
    fn from(_: crossterm::ErrorKind) -> Self { GameError{} }
}

// run_game avoids .unwrap() calls in order to ensure that control can return
// to main() before program end so the terminal cleanup code can be called
fn run_game() -> Result<(), GameError> {
    // IMMEDIATE KEYBOARD INPUT SETUP

    // STEP 1
    // create a channel for sending messages between threads
    let (ctrls_sender, ctrls_receiver) = channel::<char>();

    // STEP 2
    std::thread::spawn(move || {
        // continously wait for a single character and send it on the channel
        // this only works because we enabled raw mode
        loop {
            let mut buf = [0u8; 1]; // create a buffer for a single byte
            stdin().read_exact(&mut buf).unwrap(); // read byte into the buffer
            ctrls_sender.send(buf[0] as char).unwrap(); // send char on channel
        }
    });

    // SIMPLE EXAMPLE GAME
    // the terminal's characters are half as wide as they are tall, so the game
    // renders objects as two characters wide

    let mut player_x: i32 = 0;
    let mut player_y: i32 = 0;

    loop {
        // GAME CYCLE

        // STEP 1
        // process any controls stored in the channel
        while let Ok(ctrl) = ctrls_receiver.try_recv() {
            match ctrl {
                'w' => player_y -= 1,
                's' => player_y += 1,
                'a' => player_x -= 1,
                'd' => player_x += 1,
                'q' => return Ok(()),
                _ => (),
            }
        }
        // keep the player in the terminal
        let (term_width, term_height) = terminal::size()?;
        let world_width = term_width as i32 / 2;
        let world_height = term_height as i32;
        if player_x < 0 {
            player_x = 0;
        }
        if player_y < 0 {
            player_y = 0;
        }
        if player_x >= world_width {
            player_x = world_width - 1;
        }
        if player_y >= world_height {
            player_y = world_height - 1;
        }

        // STEP 2
        // clear the terminal
        // it's okay to do this because we're working in the alternate terminal,
        // the original terminal contents will be unaffected
        stdout()
            .queue(terminal::Clear(terminal::ClearType::All))?;

        // STEP 3
        // write some instructions in the top left :)
        // nobody should do this irl, writing the same thing every frame is
        // very inefficient
        stdout()
            .queue(style::SetForegroundColor(style::Color::White))?
            .queue(cursor::MoveTo(0, 0))?
            .write("move with wasd, press q to exit".as_bytes())?;

        // STEP 4
        // do whatever rendering needs to be done
        // in this case we move the cursor to the position indicated by player
        // x and y, set a color, and write two characters
        // .queue()? returns the the calling object, so we can chain calls
        // until .write()
        stdout()
            .queue(cursor::MoveTo(player_x as u16 * 2, player_y as u16))?
            .queue(style::SetForegroundColor(style::Color::Rgb {
                r: 255,
                g: 0,
                b: 0,
            }))?
            .write("[]".as_bytes())?;

        // STEP 5
        // since the last commands and writes were queued instead of executed,
        // we have to manually flush the output buffer
        stdout().flush()?;
    }
}