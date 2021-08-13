// From Peter Norvig’s Sudoku solver     http://www.norvig.com/sudoku.html
use ahash::RandomState;
use lazy_static::lazy_static;
use std::{collections::HashMap, fmt};

lazy_static! {
    static ref BRICKS: (Vec<char>, Vec<char>, Vec<String>, Vec<Vec<String>>) = legos();
}

fn cross(rows: &[char], cols: &[char]) -> Vec<String> {
    let mut v = Vec::with_capacity(rows.len() * cols.len());
    for ch in rows {
        for d in cols {
            v.push(format!("{}{}", ch, d));
        }
    }
    v
}

pub fn legos() -> (Vec<char>, Vec<char>, Vec<String>, Vec<Vec<String>>) {
    let cols: Vec<char> = "123456789".chars().collect();
    let rows: Vec<char> = "ABCDEFGHI".chars().collect();
    let squares = cross(&rows, &cols);
    let mut unitlist = Vec::<Vec<String>>::with_capacity(27);
    // columns
    for d in &cols {
        unitlist.push(cross(&rows, &[*d]));
    }
    // rows
    for ch in &rows {
        unitlist.push(cross(&[*ch], &cols));
    }
    // boxes
    for r in [&rows[0..3], &rows[3..6], &rows[6..9]] {
        for c in [&cols[0..3], &cols[3..6], &cols[6..9]] {
            unitlist.push(cross(r, c));
        }
    }

    (cols, rows, squares, unitlist)
}

#[derive(Debug)]
pub enum PuzzleError {
    InvalidGrid,
    Contradiction,
    Unsolved,
}

impl fmt::Display for PuzzleError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            PuzzleError::InvalidGrid => write!(f, "Invalid Grid.  Provide a string of 81 digits with 0 or . for empties."),
            PuzzleError::Contradiction => write!(f, "A contradiction occured. The puzzle is unsolvable."),
            PuzzleError::Unsolved => write!(f, "The puzzle is unsolvable."),
        }
    }
}

type PuzzleResult<T> = Result<T, PuzzleError>;
type AHashMap<K, V> = HashMap<K, V, RandomState>;

#[derive(Debug)]
pub struct Sudoku<'a> {
    cols: Vec<char>,
    rows: Vec<char>,
    squares: Vec<&'a str>,
    unitlist: Vec<Vec<&'a str>>,
    units: AHashMap<&'a str, Vec<Vec<&'a str>>>,
    peers: AHashMap<&'a str, Vec<&'a str>>,
}

impl<'a> Sudoku<'a> {
    pub fn new() -> Self {
        let (cols, rows, squares, unitlist) = (BRICKS.0.clone(), BRICKS.1.clone(), &BRICKS.2, &BRICKS.3);

        let mut squares_ref = Vec::<&str>::with_capacity(81);
        for s in squares {
            squares_ref.push(s);
        }

        let mut unitlist_ref = Vec::<Vec<&str>>::with_capacity(27);
        for u in unitlist {
            let mut unit_ref = Vec::<&str>::with_capacity(u.len());
            for s in u {
                unit_ref.push(s);
            }
            unitlist_ref.push(unit_ref);
        }

        //  units is a dictionary where each square maps to the list of units that contain the square
        let mut units = AHashMap::<&str, Vec<Vec<&str>>>::with_capacity_and_hasher(81, RandomState::default());
        for s in &squares_ref {
            let unit_s: Vec<Vec<&str>> = unitlist_ref.iter().cloned().filter(|u| u.contains(&s)).collect();
            units.insert(s, unit_s);
        }

        //  peers is a dictionary where each square s maps to the set of squares formed by the union of the squares in the units of s, but not s itself
        let mut peers = AHashMap::<&str, Vec<&str>>::with_capacity_and_hasher(81, RandomState::default());
        for s in &squares_ref {
            let mut peers_s: Vec<&str> = units[s].concat().iter().filter(|p| *p != s).map(|p| *p).collect();
            peers_s.sort();
            peers_s.dedup();
            peers.insert(s, peers_s);
        }

        Self {
            cols,
            rows,
            squares: squares_ref,
            unitlist: unitlist_ref,
            units,
            peers,
        }
    }

    fn grid_values(&self, grid: &str) -> PuzzleResult<AHashMap<&str, Vec<char>>> {
        //  Convert grid into a dict of (square, char Vec) with '0' or '.' for empties.
        let grid_chars: Vec<Vec<char>> = grid
            .chars()
            .filter(|ch| self.cols.contains(ch) || "0.".contains(*ch))
            .map(|ch| vec![ch])
            .collect();
        if grid_chars.len() == 81 {
            let mut grid_values = AHashMap::<&str, Vec<char>>::with_capacity_and_hasher(81, RandomState::default());
            grid_values.extend(self.squares.iter().map(|s| *s).zip(grid_chars.into_iter()));
            Ok(grid_values)
        } else {
            Err(PuzzleError::InvalidGrid)
        }
    }

    fn parse_grid(&self, grid: &str) -> PuzzleResult<AHashMap<&str, Vec<char>>> {
        //  Convert grid to Some dict of possible values, [square, digits], or return None if a contradiction is detected.
        let mut values = AHashMap::<&str, Vec<char>>::with_capacity_and_hasher(81, RandomState::default());
        for s in &self.squares {
            values.insert(s, self.cols.clone());
        }
        let grid_values = self.grid_values(grid)?;
        for (s, gvalues) in &grid_values {
            for d in gvalues {
                if self.cols.contains(d) && !self.assign(&mut values, s, d) {
                    return Err(PuzzleError::Contradiction);
                }
            }
        }
        Ok(values)
    }

    fn assign(&self, values: &mut AHashMap<&str, Vec<char>>, s: &str, d: &char) -> bool {
        // Assign a value d by eliminating all the other values (except d) from values[s] and propagate. Return false if a contradiction is detected.
        let other_values: Vec<char> = values[s].iter().cloned().filter(|d2| d2 != d).collect();
        other_values.iter().all(|d2| self.eliminate(values, s, d2))
    }

    fn eliminate(&self, values: &mut AHashMap<&str, Vec<char>>, s: &str, d: &char) -> bool {
        if !values[s].contains(d) {
            return true; // already eliminated
        }
        let i = values[s].iter().position(|d2| d2 == d).unwrap();
        values.get_mut(s).unwrap().remove(i);
        // (rule 1) If a square s is reduced to one value d2, then eliminate d2 from the peers.
        let d2 = values[s].clone();
        if d2.is_empty() {
            return false; // Contradiction: removed last value
        }
        if d2.len() == 1 && !self.peers[s].iter().all(|s2| self.eliminate(values, s2, &d2[0])) {
            return false;
        }
        // (rule 2) If a unit u is reduced to only one place for a value d, then put it there.
        for u in &self.units[s] {
            let dplaces: Vec<&str> = u.iter().filter(|s2| values[**s2].contains(d)).map(|s2| *s2).collect();
            if dplaces.is_empty() {
                return false; // Contradiction: no place for self value
            }
            // if d can only be in one place in unit assign it there
            if dplaces.len() == 1 && !self.assign(values, &dplaces[0], d) {
                return false;
            }
        }
        true
    }

    fn search(&self, values: AHashMap<&'a str, Vec<char>>) -> PuzzleResult<AHashMap<&str, Vec<char>>> {
        // Using depth-first search and propagation, try all possible values
        if values.iter().all(|(_, v)| v.len() == 1) {
            return Ok(values); // Solved!
        }
        // Choose the unfilled square s with the fewest possibilities
        let (_, s) = values.iter().filter(|&(_, v)| v.len() > 1).map(|(s, v)| (v.len(), s)).min().unwrap();
        for d in &values[s] {
            let mut cloned_values = values.clone();
            if self.assign(&mut cloned_values, s, d) {
                if let Ok(svalues) = self.search(cloned_values) {
                    return Ok(svalues);
                }
            }
        }
        Err(PuzzleError::Contradiction)
    }

    pub fn solve(&self, grid: &str) -> PuzzleResult<String> {
        let values = self.parse_grid(grid).and_then(|v| self.search(v))?;
        if self.solved(&values) {
            Ok(self.squares.iter().map(|s| values[s][0]).collect())
        } else {
            Err(PuzzleError::Unsolved)
        }
    }

    fn solved(&self, values: &AHashMap<&str, Vec<char>>) -> bool {
        //  A puzzle is solved if each unit is a permutation of the digits 1 to 9.
        let unitsolved = |unit: &Vec<&str>| {
            let mut digits_values = unit.iter().map(|s| values[*s].iter().collect()).collect::<Vec<String>>();
            digits_values.sort();
            digits_values == self.cols.iter().map(char::to_string).collect::<Vec<String>>()
        };
        self.unitlist.iter().all(unitsolved)
    }

    pub fn display(grid: &str) -> PuzzleResult<Vec<String>> {
        let grid_chars: Vec<char> = grid.chars().filter(|c| "123456789.0".contains(*c)).collect();
        if grid_chars.len() == 81 {
            let width = 2;
            let sep = ["-"; 3].iter().map(|c| c.repeat(3 * width)).collect::<Vec<String>>().join("+");
            let mut lines = Vec::<String>::new();
            for s in grid_chars.chunks(27) {
                for r in s.chunks(9) {
                    lines.push(
                        r.chunks(3)
                            .map(|s| s.iter().map(|c| format!("{0: ^1$}", c, width)).collect())
                            .collect::<Vec<String>>()
                            .join("|"),
                    );
                }
                lines.push(sep.clone());
            }
            lines.pop(); // to remove the last separator
            Ok(lines)
        } else {
            Err(PuzzleError::InvalidGrid)
        }
    }

    #[cfg(feature = "test")]
    pub fn random_puzzle(&self, n: usize, rng: &mut ThreadRng) -> String {
        /*  Make a random puzzle with N or more assignments. Restart on contradictions.
        Note the resulting puzzle is not guaranteed to be solvable, but empirically
        about 99.8% of them are solvable. Some have multiple solutions. */
        let mut values = HashMap::<&str, Vec<char>, RandomState>::with_capacity_and_hasher(81, RandomState::default());
        for s in &self.squares {
            values.insert(s.clone(), self.cols.clone());
        }
        let mut squares = self.squares.clone();
        squares.shuffle(rng);
        for s in &squares {
            let d2 = values[s].clone();
            if !self.assign(&mut values, s, d2.choose(rng).unwrap()) {
                break;
            }
            let mut ds: Vec<Vec<char>> = values.iter().filter(|&(_, v)| v.len() == 1).map(|(_, v)| v.clone()).collect();
            if ds.len() >= n {
                ds.sort();
                ds.dedup();
                if ds.len() >= 8 {
                    return self
                        .squares
                        .iter()
                        .map(|s| if values[s].len() == 1 { values[s][0] } else { '.' })
                        .collect();
                }
            }
        }
        self.random_puzzle(17, rng) // Give up and make a new puzzle
    }

    #[cfg(feature = "test")]
    pub fn test(&self) {
        // A set of unit tests.
        assert_eq!(self.squares.len(), 81);
        assert_eq!(self.unitlist.len(), 27);
        assert!(self.squares.iter().all(|s| self.units[s].len() == 3));
        assert!(self.squares.iter().all(|s| self.peers[s].len() == 20));
        assert_eq!(
            self.units.get("C2"),
            Some(&vec![
                vec!["A2", "B2", "C2", "D2", "E2", "F2", "G2", "H2", "I2"],
                vec!["C1", "C2", "C3", "C4", "C5", "C6", "C7", "C8", "C9"],
                vec!["A1", "A2", "A3", "B1", "B2", "B3", "C1", "C2", "C3"]
            ])
        );

        let mut peers_c2 = vec![
            "A2", "B2", "D2", "E2", "F2", "G2", "H2", "I2", "C1", "C3", "C4", "C5", "C6", "C7", "C8", "C9", "A1", "A3", "B1", "B3",
        ];
        peers_c2.sort();
        assert_eq!(self.peers.get("C2"), Some(&peers_c2));
        println!("All tests passed!");
    }
}
