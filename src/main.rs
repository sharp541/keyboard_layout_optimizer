use std::path::Path;
use std::collections::HashSet;

use keyboard_layout_optimizer::algorithms::Genetic;
use keyboard_layout_optimizer::keyboard_layout::*;
use keyboard_layout_optimizer::n_gram::NGramDB;

fn main() -> Result<(), std::io::Error> {
    let source_paths = vec![Path::new("data/ja.txt")];
    let db_path = Path::new("data/ja.db");
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
    let mut physical_layout = PhysicalLayout::new(cost_table).expect("Invalid cost table");
    physical_layout.calculate_tri_gram_cost();
    let mut normal_physical_layout =
        PhysicalLayout::new(normal_cost_table).expect("Invalid cost table");
    normal_physical_layout.calculate_tri_gram_cost();

    let usable_chars = vec![
        'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r',
        's', 't', 'u', 'v', 'w', 'x', 'y', 'z', '*', ';', '/', '+',
    ];

    let qwerty_layout = vec![
        'q', 'w', 'e', 'r', 't', 'y', 'u', 'i', 'o', 'p', 'a', 's', 'd', 'f', 'g', 'h', 'j', 'k',
        'l', ';', 'z', 'x', 'c', 'v', 'b', 'n', 'm', ',', '.', '/',
    ];
    let ohnishi_layout = vec![
        'q', 'l', 'u', ';', '.', 'f', 'w', 'r', 'y', 'p', 'e', 'i', 'a', 'o', ',', 'k', 't', 'n',
        's', 'h', 'z', 'x', 'c', 'v', '/', 'g', 'd', 'm', 'j', 'b',
    ];
    let astarte_layout = vec![
        'q', 'p', 'u', 'y', ',', 'j', 'd', 'h', 'g', 'w', 'i', 'o', 'e', 'a', '.', 'k', 't', 'n',
        's', 'r', 'z', 'x', '/', 'c', ';', 'm', 'l', 'f', 'b', 'v',
    ];

    let eucalyn_layout = vec![
        'q', 'w', ',', '.', ';', 'm', 'r', 'd', 'y', 'p', 'a', 'o', 'e', 'i', 'u', 'g', 't', 'k',
        's', 'n', 'z', 'x', 'c', 'v', 'f', 'b', 'h', 'j', 'l', '/',
    ];
    let custom_layout = vec![
        'z', 'y', 'r', 'd', 'b', 'v', 'x', 'e', 'p', 'c', // upper row
        'k', 's', 'n', 't', 'm', 'w', 'a', 'o', 'i', 'u', // middle row
        'f', 'j', 'l', 'h', 'g', 'q', '*', ';', '/', '+', // lower row
    ];

    let custom_layout_set: HashSet<char> = custom_layout.iter().cloned().collect();
    let tri_grams = n_gram_db
        .get_tri_grams(&custom_layout_set)
        .expect("Failed to get tri grams");

    let qwerty = LogicalLayout::from_usable_chars(&normal_physical_layout, qwerty_layout.clone());
    let score = qwerty.evaluate(&normal_physical_layout, &tri_grams);
    println!("qwerty score: {}", score);

    let ohnishi = LogicalLayout::from_usable_chars(&normal_physical_layout, ohnishi_layout.clone());
    let score = ohnishi.evaluate(&normal_physical_layout, &tri_grams);
    println!("ohnishi score: {}", score);

    let astarte = LogicalLayout::from_usable_chars(&normal_physical_layout, astarte_layout.clone());
    let score = astarte.evaluate(&normal_physical_layout, &tri_grams);
    println!("astarte score: {}", score);

    let eucalyn = LogicalLayout::from_usable_chars(&normal_physical_layout, eucalyn_layout.clone());
    let score = eucalyn.evaluate(&normal_physical_layout, &tri_grams);
    println!("eucalyn score: {}", score);

    let custom = LogicalLayout::from_usable_chars(&physical_layout, custom_layout.clone());
    let score = custom.evaluate(&physical_layout, &tri_grams);
    println!("custom score: {}", score);
    physical_layout.print(&custom.output());

    let algorithm = Genetic::new(32, 32);

    algorithm.optimize(&physical_layout, &usable_chars, &n_gram_db, 10000, true);

    Ok(())
}
