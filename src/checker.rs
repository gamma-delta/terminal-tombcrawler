use aglet::{Coord, CoordVec, Direction8};
use ahash::AHashSet;

use crate::{Puzzle, Solution, Tile};

macro_rules! dbgprn {
  ($doit:expr, $fmt:literal, $($args:expr),* $(,)?) => {
    if $doit {
      println!($fmt, $($args,)*);
    }
  };

  ($doit:expr, $fmt:literal) => {
    if $doit {
      println!($fmt);
    }
  }
}

impl Puzzle {
  /// - All dead ends contain a monster; all monsters are in a dead end.
  /// - Each treasure chest is in a 3x3 area with exactly one entrance.
  /// - Besides treasure rooms, there are no 2x2 corridors.
  /// - All corridors are connected.
  pub fn check_solution<S: Solution>(
    &self,
    solution: &S,
    debug: bool,
  ) -> Result<(), Failure> {
    let (chests, big_opens) = self.check_shape(solution, debug)?;

    let mut claimed_by_chests = AHashSet::new();
    for chest in chests {
      let ext = self.check_chest(solution, chest, debug)?;
      claimed_by_chests.extend(ext);
    }

    let unclaimed = big_opens.difference(&claimed_by_chests);
    // for now
    let unclaimed = unclaimed.collect::<Vec<_>>();
    if !unclaimed.is_empty() {
      dbgprn!(debug, "these were not owned: {:?}", &unclaimed);
      return Err(Failure::new(
        *unclaimed[0],
        FailureReason::LargeAreaOutsideOfTreasureRoom,
      ));
    }

    Ok(())
  }
  /// Check that:
  /// - No overlaps
  /// - Everything is contiguous
  /// - Dead end <=> monster
  ///
  /// Also return chest locations
  fn check_shape<S: Solution>(
    &self,
    solution: &S,
    debug: bool,
  ) -> Result<(AHashSet<Coord>, AHashSet<Coord>), Failure> {
    let (openings, monsters, chests) = {
      let mut openings = AHashSet::new();
      let mut monsters = AHashSet::new();
      let mut chests = AHashSet::new();

      for y in 0..self.height() {
        for x in 0..self.width() {
          let coord = Coord::new(x, y);
          if let Some(tile) = self.get_tile(coord) {
            let slot = match tile {
              Tile::Monster => &mut monsters,
              Tile::TreasureChest => &mut chests,
            };
            slot.insert(coord);
          }

          if solution.is_wall(coord) {
            if let Some(tile) = self.get_tile(coord) {
              return Err(Failure {
                reason: FailureReason::WallOverlapsFilledTile(tile),
                pos: coord,
              });
            }
          } else {
            openings.insert(coord);
          }
        }
      }
      (openings, monsters, chests)
    };

    let reachable_via_floodfill = {
      let mut rvf = AHashSet::new();
      let start = match openings.iter().next() {
        Some(it) => it,
        None => {
          // If we're here, then we know there's no walls overlapping stuff.
          // So that means there's no puzzle components and
          // it's technically correct to fill totally.
          return Ok((chests, AHashSet::new()));
        }
      };
      let mut todo = vec![*start];
      while let Some(here) = todo.pop() {
        if rvf.insert(here) {
          for n in here.neighbors4() {
            if openings.contains(&n) {
              todo.push(n);
            }
          }
        }
      }
      rvf
    };

    let mut big_opens = AHashSet::new();
    for coord in openings.iter().copied() {
      if !reachable_via_floodfill.contains(&coord) {
        return Err(Failure::new(coord, FailureReason::DiscontiguousAreas));
      }

      // To check for 2x2s we see if 3 consecutive neighbors,
      // two orthag and one diag, are empty
      'runs: for orthag in [
        Direction8::North,
        Direction8::East,
        Direction8::South,
        Direction8::West,
      ] {
        let neighbor_dirs = [orthag, orthag.rotate_by(1), orthag.rotate_by(2)];
        let too_big = neighbor_dirs.iter().all(|&nd| {
          if let Some(neighbor) = (coord.to_icoord() + nd.deltas()).to_coord() {
            let wall = neighbor.x >= self.width()
              || neighbor.y >= self.height()
              || solution.is_wall(neighbor);
            !wall
          } else {
            false
          }
        });
        if too_big {
          dbgprn!(
            debug,
            "{} marked as too big with run {:?}",
            coord,
            &neighbor_dirs,
          );
          big_opens.insert(coord);
          break 'runs;
        }
      }
      // Dead ends have 3 wall cells.
      let neighbor_count = coord
        .to_icoord()
        .neighbors4()
        .into_iter()
        .filter(|n| match n.to_coord() {
          None => true,
          Some(n) => !openings.contains(&n),
        })
        .count();
      match neighbor_count {
        0 | 1 | 2 => {
          if monsters.contains(&coord) {
            return Err(Failure::new(
              coord,
              FailureReason::MonsterWithoutDeadEnd,
            ));
          }
        }
        3 | 4 => {
          if !monsters.contains(&coord) {
            return Err(Failure::new(
              coord,
              FailureReason::DeadEndWithoutMonster,
            ));
          }
        }
        _ => unreachable!(),
      }
    }

    Ok((chests, big_opens))
  }

  fn check_chest<S: Solution>(
    &self,
    solution: &S,
    chest: Coord,
    debug: bool,
  ) -> Result<impl IntoIterator<Item = Coord>, Failure> {
    // interestinly the source code doesn't actually appear to check
    // for one entrance?
    dbgprn!(debug, "checking chest at {}", chest);
    let min_corner_x = chest.x.saturating_sub(2);
    let max_corner_x = (min_corner_x + 2).min(self.width());
    let min_corner_y = chest.y.saturating_sub(2);
    let max_corner_y = (min_corner_y + 2).min(self.height());
    dbgprn!(
      debug,
      "scanning x in {}..={}, y in {}..={}",
      min_corner_x,
      max_corner_x,
      min_corner_y,
      max_corner_y
    );
    for corner_y in min_corner_y..=max_corner_y {
      'pick_corner: for corner_x in min_corner_x..=max_corner_x {
        dbgprn!(debug, "  trying the corner to be {},{}", corner_x, corner_y);
        let mut owned = Vec::new();

        for y in corner_y..corner_y + 3 {
          for x in corner_x..corner_x + 3 {
            let here = Coord::new(x, y);
            if solution.is_wall(here) {
              // this corner is invalid womp womp
              // the src code checks for non-monster also, but they'd be ruled
              // out by the no-2x2 rule.
              dbgprn!(
                debug,
                "    there was a wall at {}, trying new corner",
                here
              );
              continue 'pick_corner;
            }
            owned.push(here);
          }
        }

        // given this corner position, search the border.
        // don't search the corners, though.
        dbgprn!(
          debug,
          "  succeeded at no wall check, checking for exactly one entrance"
        );
        let top_bottom =
          (corner_x as i32..=corner_x as i32 + 2).flat_map(|x| {
            [corner_y as i32 - 1, corner_y as i32 + 3]
              .into_iter()
              .map(move |y| CoordVec::new(x, y))
          });
        let left_right =
          (corner_y as i32..=corner_y as i32 + 2).flat_map(|y| {
            [corner_x as i32 - 1, corner_x as i32 + 3]
              .into_iter()
              .map(move |x| CoordVec::new(x, y))
          });

        let mut found_empty = false;
        for border_coord in top_bottom.chain(left_right) {
          let is_wall = match border_coord.to_coord() {
            None => true,
            Some(it) => solution.is_wall(it),
          };
          dbgprn!(
            debug,
            "    checking border pos {} (wall={})",
            border_coord,
            is_wall
          );
          if !is_wall {
            match found_empty {
              false => {
                dbgprn!(debug, "      haven't found an empty yet");
                found_empty = true;
              }
              true => {
                // this is not the spot :(
                dbgprn!(
                  debug,
                  "      have found an empty yet, trying new corner"
                );
                continue 'pick_corner;
              }
            }
          }
        }

        if found_empty {
          // yayayyayay!
          dbgprn!(
            debug,
            "succeeded at {},{}! owns {:?}",
            corner_x,
            corner_y,
            &owned
          );
          return Ok(owned);
        }
      }
    }

    Err(Failure::new(chest, FailureReason::NoTreasureRoom))
  }
}

#[derive(Debug, Clone, Copy)]
pub struct Failure {
  pub reason: FailureReason,
  pub pos: Coord,
}

impl Failure {
  pub fn new(pos: Coord, reason: FailureReason) -> Self {
    Self { reason, pos }
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureReason {
  EntirelyFilledWithWalls,
  WallOverlapsFilledTile(Tile),
  DiscontiguousAreas,
  DeadEndWithoutMonster,
  MonsterWithoutDeadEnd,
  NoTreasureRoom,
  LargeAreaOutsideOfTreasureRoom,
}
