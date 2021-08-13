pub mod solver;

#[cfg(test)]
mod tests {
    use crate::solver::Sudoku;

    #[test]
    fn solve() {
        let ss = Sudoku::new();
        assert_eq!(
            ss.solve("700000600060001070804020005000470000089000340000039000600050709010300020003000004")
                .unwrap(),
            "791543682562981473834726915356478291289615347147239568628154739415397826973862154"
        );
    }
}
