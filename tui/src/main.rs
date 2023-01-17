mod harness;

use std::fs;

use aglet::Direction8;
use argh::FromArgs;
use eyre::eyre;
use harness::SolveHarness;
use terminal_tombcrawler::Solution;

fn main() -> eyre::Result<()> {
  let args: ArgsEntrypoint = argh::from_env();

  match args.sub {
    Subcommands::Play(play) => play.run()?,
    Subcommands::TestSolver(ts) => ts.run()?,
  }

  Ok(())
}

#[derive(FromArgs, Debug)]
/// A terminal clone of Zach Barth's Dungeons and Diagrams.
struct ArgsEntrypoint {
  #[argh(subcommand)]
  sub: Subcommands,
}

#[derive(FromArgs, Debug)]
#[argh(subcommand)]
enum Subcommands {
  Play(CmdPlay),
  TestSolver(CmdTestSolver),
}

/// Play a game in the terminal.
///
/// Controls:
/// - Arrow keys or HJKL to move the cusor. Press shift to snap to the edge of
///   the grid.
/// - Q to toggle wall.
/// - W to toggle known free spaces (as a hint to you).
/// - Ctrl+C to quit.
/// - Ctrl+L to redraw the screen.
#[derive(FromArgs, Debug)]
#[argh(subcommand, name = "play")]
struct CmdPlay {
  /// path to `.ttc` file with a puzzle.
  #[argh(positional)]
  path: String,
}

impl CmdPlay {
  fn run(&self) -> eyre::Result<()> {
    let file = fs::read_to_string(&self.path)?;
    let level = terminal_tombcrawler::parse_to_level(&file)
      .map_err(|e| eyre!("{}", e.to_string()))?;
    SolveHarness::enter(level)?;
    Ok(())
  }
}

/// Temporarily test the solver
#[derive(FromArgs, Debug)]
#[argh(subcommand, name = "test-solver")]
struct CmdTestSolver {}

impl CmdTestSolver {
  fn run(&self) -> eyre::Result<()> {
    let level = terminal_tombcrawler::parse_to_level(
      "r#Test level
      
---
 52125
5.....
2.....
2..$..
2.....
4.....
0@...@
      ",
    )
    .map_err(|e| eyre!("{}", e.to_string()))?;

    struct Dummy;
    impl Solution for Dummy {
      fn is_wall(&self, coord: aglet::Coord) -> bool {
        let lookup: [[u8; 5]; 6] = [
          [1, 1, 1, 1, 1],
          [1, 0, 0, 0, 1],
          [1, 0, 0, 0, 1],
          [1, 0, 0, 0, 1],
          [1, 1, 0, 1, 1],
          [0, 0, 0, 0, 0],
        ];
        lookup[coord.y as usize][coord.x as usize] != 0
      }
    }
    let solved = level.puzzle().check_solution(&Dummy, true);
    println!("{:?}", solved);

    Ok(())
  }
}
