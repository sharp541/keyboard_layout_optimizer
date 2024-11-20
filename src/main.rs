use std::path::Path;

use keyboard_layout_optimizer::algorithms::HillClimbing;
use keyboard_layout_optimizer::interfaces::Algorithm;
use keyboard_layout_optimizer::keyboard_layout::*;

fn main() -> Result<(), std::io::Error> {
    let cost_table: [[f32; NUM_COLS]; NUM_ROWS] = [
        [3.0, 2.4, 2.0, 2.2, 3.2, 3.2, 2.2, 2.0, 2.4, 3.0], // 上段
        [1.6, 1.3, 1.1, 1.0, 2.9, 2.9, 1.0, 1.1, 1.3, 1.6], // 中段（ホームポジション）
        [3.2, 2.6, 2.3, 1.6, 3.0, 3.0, 1.6, 10e10, 10e10, 3.2], // 下段
    ];
    let usable_chars = vec![
        'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r',
        's', 't', 'u', 'v', 'w', 'x', 'y', 'z',
    ];

    let physical_layout = PhysicalLayout::new(cost_table).expect("Invalid cost table");
    let mut logical_layout = LogicalLayout::new(&physical_layout, usable_chars);
    let algorithm: Box<dyn Algorithm<1>> = Box::new(HillClimbing {});
    println!("Initial: {}", logical_layout);

    let text_path = Path::new("data/jap-n.txt");
    let text = std::fs::read_to_string(text_path).expect("Failed to read text file");

    algorithm.optimize(&mut logical_layout, &text, 100, 1024);

    println!("Optimized: {}", logical_layout);

    Ok(())
}
