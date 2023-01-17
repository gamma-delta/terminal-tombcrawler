//! Solver harness

use std::io::{self, Stdout, Write};

use aglet::{Coord, Direction4, Grid};
use crossterm::{
  cursor::MoveTo,
  event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
  style::{
    Attribute, Attributes, Color, Colors, Print, ResetColor, SetAttributes,
    SetColors, SetForegroundColor,
  },
  terminal::{
    disable_raw_mode, enable_raw_mode, Clear, ClearType, EnterAlternateScreen,
    LeaveAlternateScreen,
  },
  QueueableCommand,
};
use terminal_tombcrawler::{checker::Failure, Level, Solution, Tile};

const START_X: u16 = 2;
const START_Y: u16 = 2;

const TILE_STRIDE_X: u16 = 2;
const TILE_STRIDE_Y: u16 = 2;

/// This defines the position the HINTS are drawn at;
/// the board is drawn one span below.
const BOARD_X: u16 = 4;
const BOARD_Y: u16 = 6;

pub struct SolveHarness {
  level: Level,
  cursor: Coord,

  markings: Grid<Marking>,

  solved: SolvedState,

  must_redraw: bool,
}

impl SolveHarness {
  /// Transfer runtime to the harness.
  /// This will only return once the player is through.
  pub fn enter(level: Level) -> io::Result<()> {
    let markings = Grid::new(level.puzzle().width(), level.puzzle().height());

    let mut harness = Self {
      level,
      cursor: Coord::new(0, 0),
      markings,
      solved: SolvedState::JustStarted,
      must_redraw: false,
    };

    harness.spin()?;

    Ok(())
  }

  fn spin(&mut self) -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    stdout.queue(EnterAlternateScreen)?.flush()?;

    loop {
      self.draw(&mut stdout)?;

      match event::read()? {
        Event::Key(ev) => {
          if matches!(ev.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
            let quit = self.update(ev.code, ev.modifiers)?;
            if quit {
              break;
            }
          }
        }
        _ => {}
      }
    }

    stdout.queue(LeaveAlternateScreen)?.flush()?;
    disable_raw_mode()?;

    Ok(())
  }

  /// return whether to quit
  fn update(&mut self, key: KeyCode, mods: KeyModifiers) -> io::Result<bool> {
    let quit = 'inner: {
      if key == KeyCode::Char('c') && mods.contains(KeyModifiers::CONTROL) {
        break 'inner true;
      }

      if self.must_redraw {
        self.must_redraw = false;
      }
      if key == KeyCode::Char('l') && mods.contains(KeyModifiers::CONTROL) {
        self.must_redraw = true;
        break 'inner false;
      }

      let width = self.level.puzzle().width();
      let height = self.level.puzzle().height();

      let cursor_delta = match key {
        KeyCode::Left | KeyCode::Char('h') | KeyCode::Char('H') => {
          Some(Direction4::West)
        }
        KeyCode::Right | KeyCode::Char('l') | KeyCode::Char('L') => {
          Some(Direction4::East)
        }
        KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
          Some(Direction4::North)
        }
        KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
          Some(Direction4::South)
        }
        _ => None,
      };
      if let Some(cursor_delta) = cursor_delta {
        let x2 = match (cursor_delta, mods.contains(KeyModifiers::SHIFT)) {
          (Direction4::North | Direction4::South, _) => self.cursor.x,
          (Direction4::West, false) => {
            // do subtraction by wrapping around, thanks finite fields
            (self.cursor.x + width - 1).rem_euclid(width)
          }
          (Direction4::West, true) => 0,
          (Direction4::East, false) => (self.cursor.x + 1).rem_euclid(width),
          (Direction4::East, true) => width - 1,
        };
        let y2 = match (cursor_delta, mods.contains(KeyModifiers::SHIFT)) {
          (Direction4::West | Direction4::East, _) => self.cursor.y,
          (Direction4::North, false) => {
            (self.cursor.y + height - 1).rem_euclid(height)
          }
          (Direction4::North, true) => 0,
          (Direction4::South, false) => (self.cursor.y + 1).rem_euclid(height),
          (Direction4::South, true) => height - 1,
        };

        self.cursor = Coord::new(x2, y2);
        break 'inner false;
      }

      // Try markings
      if self.level.puzzle().get_tile(self.cursor).is_none() {
        let marking_here = self.markings.get(self.cursor).copied();
        let wanted_marking = match key {
          KeyCode::Char('q') => Ok(if marking_here == Some(Marking::Wall) {
            None
          } else {
            Some(Marking::Wall)
          }),
          KeyCode::Char('w') => Ok(if marking_here == None {
            Some(Marking::Empty)
          } else {
            None
          }),
          _ => Err(()),
        };
        if let Ok(marking2) = wanted_marking {
          self.markings.insert_direct(self.cursor, marking2);
          break 'inner false;
        }
      }

      false
    };
    let view = SolutionView {
      marks: &self.markings,
    };
    let solved = self.level.puzzle().check_solution(&view);
    self.solved = match solved {
      Ok(()) => SolvedState::Success,
      Err(fail) => SolvedState::Fail(fail),
    };
    Ok(quit)
  }

  fn draw(&self, stdout: &mut Stdout) -> io::Result<()> {
    if self.must_redraw {
      stdout.queue(Clear(ClearType::All))?;
    }

    stdout.queue(MoveTo(START_X, START_Y))?;
    stdout
      .queue(ResetColor)?
      .queue(Print(&self.level.title()))?;

    let (col_counts, row_counts) = self.col_row_wall_counts();
    for (x, &hint) in self.level.puzzle().top_hints().iter().enumerate() {
      let col_count = col_counts[x] as u8;
      let color = if col_count == hint {
        Color::DarkGreen
      } else if col_count > hint {
        Color::Red
      } else {
        Color::White
      };

      stdout
        .queue(MoveTo(BOARD_X + (x as u16 + 1) * TILE_STRIDE_X, BOARD_Y))?
        .queue(SetForegroundColor(color))?
        .queue(Print(hint))?;
    }
    for (y, &hint) in self.level.puzzle().side_hints().iter().enumerate() {
      let row_count = row_counts[y] as u8;
      let color = if row_count == hint {
        Color::DarkGreen
      } else if row_count > hint {
        Color::Red
      } else {
        Color::White
      };
      stdout
        .queue(MoveTo(BOARD_X, BOARD_Y + (y as u16 + 1) * TILE_STRIDE_Y))?
        .queue(SetForegroundColor(color))?
        .queue(Print(hint))?;
    }

    for y in 0..self.level.puzzle().height() {
      for x in 0..self.level.puzzle().width() {
        let coord = Coord::new(x as _, y as _);

        let (ch, cols, fmt) =
          if let Some(tile) = self.level.puzzle().get_tile(coord) {
            puzzle_tile_display(tile)
          } else if let Some(marking) = self.markings.get(coord) {
            marking.display()
          } else {
            bg_display()
          };
        let screenpos = grid_to_screen(coord);
        stdout
          .queue(MoveTo(screenpos.0, screenpos.1))?
          .queue(SetColors(cols))?
          .queue(SetAttributes(fmt))?
          .queue(Print(ch))?;
      }
    }

    // Temp
    let rightmost = grid_to_screen(Coord::new(self.level.puzzle().width(), 1));
    match self.solved {
      SolvedState::JustStarted => {}
      SolvedState::Fail(ref ono) => {
        stdout
          .queue(MoveTo(rightmost.0, rightmost.1))?
          .queue(ResetColor)?
          .queue(Print(format!("{:?}", ono.reason)))?
          .queue(MoveTo(rightmost.0, rightmost.1 + 1))?
          .queue(Print(ono.pos))?;
      }
      SolvedState::Success => {
        stdout
          .queue(MoveTo(rightmost.0, rightmost.1))?
          .queue(SetForegroundColor(Color::Green))?
          .queue(Print("yay!"))?;
      }
    }
    stdout
      .queue(MoveTo(rightmost.0, rightmost.1 + 2))?
      .queue(ResetColor)?
      .queue(Print(format!("{:?}", col_counts)))?
      .queue(MoveTo(rightmost.0, rightmost.1 + 3))?
      .queue(Print(format!("{:?}", row_counts)))?;

    let cursorpos = grid_to_screen(self.cursor);
    stdout.queue(MoveTo(cursorpos.0, cursorpos.1))?;

    stdout.flush()?;
    Ok(())
  }

  fn col_row_wall_counts(&self) -> (Vec<usize>, Vec<usize>) {
    // i recognize there's some O(n) way to do this but i don't care
    let col_counts = (0..self.level.puzzle().width())
      .map(|x| {
        (0..self.level.puzzle().height())
          .filter(|&y| {
            self.markings.get(Coord::new(x as _, y as _)).copied()
              == Some(Marking::Wall)
          })
          .count()
      })
      .collect();
    let row_counts = (0..self.level.puzzle().height())
      .map(|y| {
        (0..self.level.puzzle().width())
          .filter(|&x| {
            self.markings.get(Coord::new(x as _, y as _)).copied()
              == Some(Marking::Wall)
          })
          .count()
      })
      .collect();

    (col_counts, row_counts)
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Marking {
  Wall,
  Empty,
}

impl Marking {
  fn display(&self) -> (char, Colors, Attributes) {
    match self {
      Marking::Wall => (
        '#',
        Colors::new(Color::White, Color::DarkGrey),
        Attribute::Bold.into(),
      ),
      Marking::Empty => (
        '*',
        Colors::new(Color::DarkMagenta, Color::Reset),
        Attribute::Italic.into(),
      ),
    }
  }
}

enum SolvedState {
  JustStarted,
  /// Temporarily display to the player
  Fail(Failure),
  Success,
}

fn puzzle_tile_display(tile: Tile) -> (char, Colors, Attributes) {
  match tile {
    Tile::Monster => (
      '@',
      Colors::new(Color::Red, Color::Reset),
      Attributes::default() | Attribute::Bold | Attribute::NoItalic,
    ),
    Tile::TreasureChest => (
      '$',
      Colors::new(Color::Yellow, Color::Reset),
      Attributes::default() | Attribute::Bold | Attribute::NoItalic,
    ),
  }
}

fn bg_display() -> (char, Colors, Attributes) {
  (
    '.',
    Colors::new(Color::DarkGrey, Color::Reset),
    Attribute::NormalIntensity.into(),
  )
}

fn grid_to_screen(coord: Coord) -> (u16, u16) {
  (
    (coord.x as u16 + 1) * TILE_STRIDE_X + BOARD_X,
    (coord.y as u16 + 1) * TILE_STRIDE_Y + BOARD_Y,
  )
}

struct SolutionView<'a> {
  marks: &'a Grid<Marking>,
}

impl<'a> Solution for SolutionView<'a> {
  fn is_wall(&self, coord: Coord) -> bool {
    match self.marks.get(coord) {
      None => false,
      Some(mark) => *mark == Marking::Wall,
    }
  }
}
