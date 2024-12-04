use std::path::Path;

use keyboard_layout_optimizer::algorithms::{Annealing, Genetic, HillClimbing};
use keyboard_layout_optimizer::keyboard_layout::*;
use keyboard_layout_optimizer::n_gram::NGramDB;

fn main() -> Result<(), std::io::Error> {
    let cost_table: [[f32; NUM_COLS]; NUM_ROWS] = [
        [3.7, 2.4, 2.0, 2.2, 3.2, 3.2, 2.2, 2.0, 2.4, 3.7], // 上段
        [3.0, 1.3, 1.1, 1.0, 1.6, 1.6, 1.0, 1.1, 1.3, 3.0], // 中段（ホームポジション）
        [3.2, 2.6, 2.3, 1.6, 3.0, 3.0, 1.6, 10e10, 10e10, 3.2], // 下段
    ];
    let usable_chars = vec![
        'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r',
        's', 't', 'u', 'v', 'w', 'x', 'y', 'z', ',', '.',
    ];

    let physical_layout = PhysicalLayout::new(cost_table).expect("Invalid cost table");
    let mut logical_layout = LogicalLayout::from_usable_chars(&physical_layout, usable_chars);
    let algorithm = Annealing::new(100.0, 0.99);

    let source_path = Path::new("data/jap-n.txt");
    let db_path = Path::new("data/jap-n.db");
    if !db_path.exists() {
        let _ = NGramDB::new(source_path, db_path).expect("Failed to create NGramDB");
    }
    let n_gram_db = NGramDB::load(db_path).expect("Failed to load NGramDB");

    algorithm.optimize(&mut logical_layout, &n_gram_db, 10000);

    println!("Optimized:");
    physical_layout.print(&logical_layout.output());
    Ok(())
}
