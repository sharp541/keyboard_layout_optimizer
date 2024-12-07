use std::path::Path;

use keyboard_layout_optimizer::algorithms::{Annealing, Genetic, HillClimbing};
use keyboard_layout_optimizer::keyboard_layout::*;
use keyboard_layout_optimizer::n_gram::NGramDB;

fn main() -> Result<(), std::io::Error> {
    let source_paths = vec![Path::new("data/jap-n.txt")];
    let db_path = Path::new("data/jap-n.db");
    if !db_path.exists() {
        let _ = NGramDB::new(&source_paths, db_path).expect("Failed to create NGramDB");
    }
    let n_gram_db = NGramDB::load(db_path).expect("Failed to load NGramDB");

    let cost_table: [[f32; NUM_COLS]; NUM_ROWS] = [
        [3.7, 2.4, 2.0, 2.2, 3.2, 3.2, 2.2, 2.0, 2.4, 3.7], // 上段
        [3.0, 1.3, 1.1, 1.0, 1.6, 1.6, 1.0, 1.1, 1.3, 3.0], // 中段（ホームポジション）
        [3.2, 3.6, 2.3, 1.6, 3.0, 3.0, 1.6, 10e10, 10e10, 3.2], // 下段
    ];
    let physical_layout = PhysicalLayout::new(cost_table).expect("Invalid cost table");
    let qwerty_layout = vec![
        'q', 'w', 'e', 'r', 't', 'y', 'u', 'i', 'o', 'p', 'a', 's', 'd', 'f', 'g', 'h', 'j', 'k',
        'l', ';', 'z', 'x', 'c', 'v', 'b', 'n', 'm', ',', '.', '/',
    ];
    let qwerty = LogicalLayout::from_usable_chars(&physical_layout, qwerty_layout.clone());
    let tri_grams = n_gram_db
        .get_tri_grams()
        .expect("Failed to evaluate qwerty");
    let score = qwerty.evaluate(&tri_grams);
    println!("qwerty score: {}", score);

    let usable_chars = vec![
        'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r',
        's', 't', 'u', 'v', 'w', 'x', 'y', 'z', ',', '.',
    ];

    // let mut logical_layout = LogicalLayout::from_usable_chars(&physical_layout, usable_chars);
    let algorithm = Genetic::new(128);

    algorithm.optimize(&physical_layout, &qwerty_layout, &n_gram_db, 100);

    // println!("Optimized:");
    // physical_layout.print(&logical_layout.output());
    Ok(())
}
