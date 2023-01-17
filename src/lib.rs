pub mod checker;
mod parse;

pub use parse::parse_to_level;

use aglet::{Coord, Grid};

#[derive(Debug, Clone)]
pub struct Level {
  puzzle: Puzzle,
  title: String,
}

impl Level {
  pub fn new(puzzle: Puzzle, title: String) -> Self {
    Self { puzzle, title }
  }

  pub fn puzzle(&self) -> &Puzzle {
    &self.puzzle
  }

  pub fn title(&self) -> &str {
    &self.title
  }
}

#[derive(Debug, Clone)]
pub struct Puzzle {
  tiles: Grid<Tile>,
  top_hints: Vec<u8>,
  side_hints: Vec<u8>,
}

impl Puzzle {
  pub fn new(
    tiles: Grid<Tile>,
    top_hints: Vec<u8>,
    side_hints: Vec<u8>,
  ) -> Self {
    Self {
      tiles,
      top_hints,
      side_hints,
    }
  }

  pub fn width(&self) -> u32 {
    self.tiles.width()
  }

  pub fn height(&self) -> u32 {
    self.tiles.height()
  }

  pub fn get_tile(&self, coord: Coord) -> Option<Tile> {
    self.tiles.get(coord).copied()
  }

  pub fn top_hints(&self) -> &[u8] {
    &self.top_hints
  }

  pub fn side_hints(&self) -> &[u8] {
    &self.side_hints
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tile {
  Monster,
  TreasureChest,
}

/// An attempt to find a solution to a puzzle.
///
/// This is a trait so we can send the solution zero-copy.
pub trait Solution {
  fn is_wall(&self, coord: Coord) -> bool;
}
