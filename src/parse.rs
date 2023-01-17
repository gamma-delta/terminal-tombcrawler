use aglet::{Coord, Grid};
use nom::{
  branch::alt,
  bytes::complete::{take, take_until, take_while},
  character::complete::{
    char, line_ending, multispace0, not_line_ending, satisfy, space0,
  },
  combinator::{eof, map, opt, value},
  error::{context, VerboseError},
  multi::{count, many0},
  sequence::{terminated, tuple},
  Finish, IResult, Parser,
};

use crate::{Level, Puzzle, Tile};

/// Parse a string into a level.
pub fn parse_to_level(s: &str) -> Result<Level, VerboseError<&str>> {
  let (s, level) = level(s).finish()?;
  debug_assert_eq!(s, "");
  Ok(level)
}

fn level(s: &str) -> IResult<&str, Level, VerboseError<&str>> {
  let (s, title) = header(s)?;
  let (s, puzzle) = puzzle(s)?;
  let (s, _) = eof(s)?;
  Ok((s, Level::new(puzzle, title)))
}

/// Returns the title
fn header(s: &str) -> IResult<&str, String, VerboseError<&str>> {
  let (s, title) = terminated(not_line_ending, line_ending)(s)?;

  let (s, _comment) =
    discard_ws_after(terminated(take_until("---"), take(3usize)))(s)?;
  Ok((s, title.to_string()))
}

fn puzzle(s: &str) -> IResult<&str, Puzzle, VerboseError<&str>> {
  let (s, _corner) = char(' ')(s)?;
  let (s, top_hints) =
    discard_ws_after(take_while(|c: char| c.is_ascii_digit()))(s)?;
  let (s, puzzle_lines) = many0(|s| puzzle_line(s, top_hints.len()))(s)?;
  let (s, _trail) = multispace0(s)?;

  // and convert
  let top_hints = top_hints
    .chars()
    .map(|c| c.to_digit(10).unwrap() as u8)
    .collect::<Vec<_>>();
  let mut grid = Grid::new(top_hints.len() as u32, puzzle_lines.len() as u32);
  let mut side_hints = Vec::new();
  for (y, pl) in puzzle_lines.into_iter().enumerate() {
    side_hints.push(pl.hint);
    for (x, tile) in pl.tiles.into_iter().enumerate() {
      if let Some(tile) = tile {
        let coord = Coord::new(x as _, y as _);
        grid.insert(coord, tile);
      }
    }
  }

  Ok((s, Puzzle::new(grid, top_hints, side_hints)))
}

fn puzzle_line(
  s: &str,
  len: usize,
) -> IResult<&str, PuzzleLine, VerboseError<&str>> {
  let (s, hint) = map(satisfy(|c| c.is_ascii_digit()), |c| {
    c.to_digit(10).unwrap() as u8
  })(s)?;
  let (s, tiles) = discard_ws_after(count(a_tile, len))(s)?;
  Ok((s, PuzzleLine { hint, tiles }))
}

fn a_tile(s: &str) -> IResult<&str, Option<Tile>, VerboseError<&str>> {
  context(
    "tile",
    alt((
      value(Some(Tile::Monster), char('@')),
      value(Some(Tile::TreasureChest), char('$')),
      value(None, char('.')),
    )),
  )(s)
}

// nice combinator
fn discard_ws_after<'a, O, F>(
  inner: F,
) -> impl FnMut(&'a str) -> IResult<&'a str, O, VerboseError<&'a str>>
where
  F: Parser<&'a str, O, VerboseError<&'a str>>,
{
  terminated(inner, tuple((space0, opt(line_ending))))
}

struct PuzzleLine {
  hint: u8,
  tiles: Vec<Option<Tile>>,
}
