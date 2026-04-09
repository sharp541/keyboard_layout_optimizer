use std::env;
use std::path::Path;

use keyboard_layout_optimizer::algorithms::{Genetic, OptimizeConfig};
use keyboard_layout_optimizer::keyboard_layout::Finger as F;
use keyboard_layout_optimizer::keyboard_layout::*;
use keyboard_layout_optimizer::n_gram::{NGramDB, NGramSource, SourceKind};

fn parse_weights_from_args() -> Result<(f32, f32), String> {
    let mut ja_weight = 0.5_f32;
    let mut en_weight = 0.5_f32;

    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--ja-weight" => {
                let value = args
                    .next()
                    .ok_or_else(|| "--ja-weight requires a numeric value".to_string())?;
                ja_weight = value
                    .parse::<f32>()
                    .map_err(|_| format!("invalid --ja-weight value: {value}"))?;
            }
            "--en-weight" => {
                let value = args
                    .next()
                    .ok_or_else(|| "--en-weight requires a numeric value".to_string())?;
                en_weight = value
                    .parse::<f32>()
                    .map_err(|_| format!("invalid --en-weight value: {value}"))?;
            }
            _ => {
                return Err(format!(
                    "unknown argument: {arg} (supported: --ja-weight <f32>, --en-weight <f32>)"
                ));
            }
        }
    }

    if ja_weight < 0.0 || en_weight < 0.0 {
        return Err("weights must be non-negative".to_string());
    }

    let sum = ja_weight + en_weight;
    if sum <= f32::EPSILON {
        return Err("sum of --ja-weight and --en-weight must be > 0".to_string());
    }

    Ok((ja_weight / sum, en_weight / sum))
}

fn main() -> Result<(), std::io::Error> {
    let (ja_weight, en_weight) = parse_weights_from_args()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;

    let source_paths = [Path::new("data/ja.txt"), Path::new("data/en.txt")];
    let sources = vec![
        NGramSource::new("ja", SourceKind::Japanese, source_paths[0]),
        NGramSource::new("en", SourceKind::English, source_paths[1]),
    ];
    let db_path = Path::new("data/ja_en.db");
    if NGramDB::requires_rebuild(db_path).expect("Failed to inspect NGramDB schema") {
        let _ = NGramDB::new(&sources, db_path).expect("Failed to create NGramDB");
    }
    let n_gram_db = NGramDB::load(db_path).expect("Failed to load NGramDB");

    if let Some((ja_ratio, en_ratio)) = n_gram_db
        .get_source_size_ratios()
        .expect("Failed to get source ratios")
    {
        println!(
            "stored source size ratio (ja/en): {:.4} / {:.4}",
            ja_ratio, en_ratio
        );
    }

    println!(
        "evaluation weights (ja/en): {:.4} / {:.4}",
        ja_weight, en_weight
    );

    let cost_table: [f32; NUM_COLS * NUM_ROWS] = [
        2.2, 1.0, 1.0, 2.8, 10e10, 10e10, 2.8, 1.0, 1.02, 2.2, // upper row
        1.3, 1.0, 1.0, 1.0, 2.5, 2.5, 1.0, 1.0, 1.0, 1.3, // middle row
        10e10, 10e10, 10e10, 1.6, 10e10, 10e10, 1.6, 10e10, 10e10, 10e10, // lower row
    ];
    let finger_table: [F; NUM_COLS * NUM_ROWS] = [
        F::R,
        F::R,
        F::M,
        F::M,
        F::I,
        F::I,
        F::M,
        F::M,
        F::R,
        F::R, // upper row
        F::P,
        F::R,
        F::M,
        F::I,
        F::I,
        F::I,
        F::I,
        F::M,
        F::R,
        F::P, // middle row
        F::P,
        F::R,
        F::M,
        F::I,
        F::I,
        F::I,
        F::I,
        F::M,
        F::R,
        F::P, // lower row
    ];
    let mut physical_layout =
        PhysicalLayout::new(cost_table, finger_table).expect("Invalid cost table");
    physical_layout.calculate_tri_gram_cost();

    let custom_layout = vec![
        'h', 'k', 'r', 'z', 'q', '.', ',', 'e', 'p', 'v', // upper row
        'm', 's', 'n', 't', 'g', 'c', 'a', 'o', 'i', 'u', // middle row
        'y', 'b', 'l', 'd', 'j', 'x', 'f', 'w', // lower row
    ];

    let custom = LogicalLayout::from_usable_chars(custom_layout.as_ref());
    let split_tri_grams = n_gram_db
        .get_split_tri_grams_for_layout(|c| custom.resolve_char_index(c).is_some())
        .expect("Failed to get split tri grams");
    let ja_score = custom.evaluate(&physical_layout, &split_tri_grams.japanese);
    let en_score = custom.evaluate(&physical_layout, &split_tri_grams.english);
    let score = ja_weight * ja_score + en_weight * en_score;
    println!(
        "custom score: {} (ja: {}, en: {})",
        score, ja_score, en_score
    );
    custom.print();

    let algorithm = Genetic::new(48, 24);

    algorithm.optimize(
        &physical_layout,
        &custom_layout,
        &n_gram_db,
        OptimizeConfig {
            iterations: 60000,
            shuffle: true,
            early_stop_count: 8000,
            ja_weight,
            en_weight,
        },
    );

    Ok(())
}
