use aglet::{Coord, CoordVec};
use ahash::AHashSet;

use crate::{Puzzle, Solution, Tile};

impl Puzzle {
  /// - All dead ends contain a monster; all monsters are in a dead end.
  /// - Each treasure chest is in a 3x3 area with exactly one entrance.
  /// - Besides treasure rooms, there are no 2x2 corridors.
  /// - All corridors are connected.
  pub fn check_solution<S: Solution>(
    &self,
    solution: &S,
  ) -> Result<(), Failure> {
    let (chests, big_opens) = self.check_shape(solution)?;

    let mut claimed_by_chests = AHashSet::new();
    for chest in chests {
      let ext = self.check_chest(solution, chest)?;
      claimed_by_chests.extend(ext);
    }

    if let Some(unclaimed_open) =
      big_opens.difference(&claimed_by_chests).next()
    {
      return Err(Failure::new(
        *unclaimed_open,
        FailureReason::LargeAreaOutsideOfTreasureRoom,
      ));
    }

    Ok(())
  }

  fn check_chest<S: Solution>(
    &self,
    solution: &S,
    chest: Coord,
  ) -> Result<impl IntoIterator<Item = Coord>, Failure> {
    // interestinly the source code doesn't actually appear to check
    // for one entrance?
    let min_corner_x = chest.x.saturating_sub(2);
    let max_corner_x = (min_corner_x + 2).min(self.width());
    let min_corner_y = chest.y.saturating_sub(2);
    let max_corner_y = (min_corner_y + 2).min(self.height());
    for corner_y in min_corner_y..=max_corner_y {
      'pick_corner: for corner_x in min_corner_x..=max_corner_x {
        let mut owned = Vec::new();

        for y in corner_y..corner_y + 3 {
          for x in corner_x..corner_x + 3 {
            let here = Coord::new(x, y);
            if solution.is_wall(here) {
              // this corner is invalid womp womp
              // the src code checks for non-monster also, but they'd be ruled
              // out by the no-2x2 rule.
              continue 'pick_corner;
            }
            owned.push(here);
          }
        }

        // given this corner position, search the border.
        // don't search the corners, though.
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
          if is_wall {
            match found_empty {
              false => {
                found_empty = true;
              }
              true => {
                // this is not the spot :(
                continue 'pick_corner;
              }
            }
          }
        }

        if found_empty {
          // yayayyayay!
          return Ok(owned);
        }
      }
    }

    Err(Failure::new(chest, FailureReason::NoTreasureRoom))
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

      // To check for 2x2s we see if 3 consecutive neighbors
      // (including diagonals) are empty.
      'runs: for run in coord.neighbors8().windows(3) {
        if run.iter().all(|n| openings.contains(n)) {
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
