use std::collections::HashSet;
use std::path::Path;

use keyboard_layout_optimizer::algorithms::Genetic;
use keyboard_layout_optimizer::keyboard_layout::Finger as F;
use keyboard_layout_optimizer::keyboard_layout::*;
use keyboard_layout_optimizer::n_gram::NGramDB;

fn main() -> Result<(), std::io::Error> {
    let source_paths = vec![Path::new("data/ja.txt"), Path::new("data/en.txt")];
    let db_path = Path::new("data/ja_en.db");
    if !db_path.exists() {
        let _ = NGramDB::new(&source_paths, db_path).expect("Failed to create NGramDB");
    }
    let n_gram_db = NGramDB::load(db_path).expect("Failed to load NGramDB");

    let cost_table: [[f32; NUM_COLS]; NUM_ROWS] = [
        [2.5, 1.5, 1.4, 2.8, 3.6, 3.6, 2.8, 1.4, 1.5, 2.5],
        [2.0, 1.3, 1.1, 1.0, 2.2, 2.2, 1.0, 1.1, 1.3, 2.0],
        [3.2, 3.6, 2.7, 1.6, 3.2, 3.2, 1.6, 10e10, 10e10, 3.2],
    ];
    let normal_cost_table: [[f32; NUM_COLS]; NUM_ROWS] = [
        [3.5, 2.4, 2.0, 2.2, 3.2, 3.2, 2.2, 2.0, 2.4, 3.5],
        [1.6, 1.3, 1.1, 1.0, 2.0, 2.0, 1.0, 1.1, 1.3, 2.0],
        [3.2, 3.6, 2.3, 1.6, 3.0, 3.0, 1.6, 2.3, 3.6, 3.2],
    ];
    let finger_table: [[F; NUM_COLS]; NUM_ROWS] = [
        [F::R, F::R, F::M, F::M, F::I, F::I, F::M, F::M, F::R, F::R],
        [F::P, F::R, F::M, F::I, F::I, F::I, F::I, F::M, F::R, F::P],
        [F::P, F::R, F::M, F::I, F::I, F::I, F::I, F::M, F::R, F::P],
    ];
    let mut physical_layout =
        PhysicalLayout::new(cost_table, finger_table).expect("Invalid cost table");
    physical_layout.calculate_tri_gram_cost();
    let mut normal_physical_layout =
        PhysicalLayout::new(normal_cost_table, finger_table).expect("Invalid cost table");
    normal_physical_layout.calculate_tri_gram_cost();

    let custom_layout = vec![
        'h', 'k', 'r', 'z', 'q', '+', ';', 'e', 'p', 'v', // upper row
        'm', 's', 'n', 't', 'g', 'c', 'a', 'o', 'i', 'u', // middle row
        'y', 'b', 'l', 'd', 'j', 'x', 'f', '*', '/', 'w', // lower row
    ];

    let custom_layout_set: HashSet<char> = custom_layout.iter().cloned().collect();
    let tri_grams = n_gram_db
        .get_tri_grams(&custom_layout_set)
        .expect("Failed to get tri grams");

    let custom = LogicalLayout::from_usable_chars(&physical_layout, custom_layout.clone());
    let score = custom.evaluate(&physical_layout, &tri_grams);
    println!("custom score: {}", score);
    physical_layout.print(&custom.output());

    // let algorithm = Genetic::new(32, 16);

    // algorithm.optimize(&physical_layout, &usable_chars, &n_gram_db, 40000, true, 3000);

    Ok(())
}
