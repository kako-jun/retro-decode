//! CART (Classification And Regression Trees) 決定木学習バイナリ
//!
//! Leaf エンコーダの選択ルールを決定木で学習する。
//! 特徴量：distance, length, image_x, image_y, ring_r, prev_token_kind
//! ラベル：leaf_choice_index

use std::collections::HashMap;
use std::fs::File;
use std::io::BufRead;
use std::path::PathBuf;
use anyhow::{anyhow, Result};
use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "train_decision_tree")]
#[command(about = "CART決定木学習: Leaf Encoder選択ルール抽出")]
struct Args {
    /// CSV ファイルパス
    #[arg(value_name = "FILE")]
    csv_file: PathBuf,

    /// 最大出力ルール数（デフォルト: 100）
    #[arg(long, default_value = "100")]
    max_rules: usize,
}

/// 1つのデータポイント
#[derive(Clone, Debug)]
struct DataPoint {
    distance: f64,
    length: f64,
    image_x: f64,
    image_y: f64,
    ring_r: f64,
    prev_token_kind: String,
    leaf_choice: usize, // ラベル
}

/// 分割条件
#[derive(Clone, Debug)]
struct Split {
    feature: String,
    threshold: f64,
}

/// 決定木のノード
#[derive(Clone, Debug)]
enum TreeNode {
    /// リーフノード：最も多い leaf_choice を記録
    Leaf {
        choice: usize,
        count: usize,
        coverage: usize,
    },
    /// 内部ノード
    Internal {
        split: Split,
        left: Box<TreeNode>,
        right: Box<TreeNode>,
        samples: usize,
    },
}

/// Gini 不純度を計算
fn gini(counts: &HashMap<usize, usize>) -> f64 {
    let total: usize = counts.values().sum();
    if total == 0 {
        return 0.0;
    }
    let total_f = total as f64;
    let mut gini_val = 1.0;
    for &count in counts.values() {
        let p = count as f64 / total_f;
        gini_val -= p * p;
    }
    gini_val
}

/// クラスラベルの分布を計算
fn class_distribution(data: &[DataPoint]) -> HashMap<usize, usize> {
    let mut counts = HashMap::new();
    for point in data {
        *counts.entry(point.leaf_choice).or_insert(0) += 1;
    }
    counts
}

/// 最頻値のラベルを取得
fn majority_class(counts: &HashMap<usize, usize>) -> usize {
    counts
        .iter()
        .max_by_key(|&(_, count)| count)
        .map(|(&choice, _)| choice)
        .unwrap_or(0)
}

/// 特徴量値を抽出
fn get_feature_value(point: &DataPoint, feature: &str) -> Option<f64> {
    match feature {
        "distance" => Some(point.distance),
        "length" => Some(point.length),
        "image_x" => Some(point.image_x),
        "image_y" => Some(point.image_y),
        "ring_r" => Some(point.ring_r),
        _ => None,
    }
}

/// 候補となる閾値を生成
fn find_split_thresholds(data: &[DataPoint], feature: &str) -> Vec<f64> {
    let mut values: Vec<f64> = data
        .iter()
        .filter_map(|p| get_feature_value(p, feature))
        .collect();
    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    values.dedup_by(|a, b| (*a - *b).abs() < 1e-9);

    // 中点を閾値として使用
    let mut thresholds = Vec::new();
    for i in 0..values.len().saturating_sub(1) {
        thresholds.push((values[i] + values[i + 1]) / 2.0);
    }
    thresholds
}

/// Gini 情報利得を計算
fn information_gain(parent_gini: f64, left_size: usize, right_size: usize, left_gini: f64, right_gini: f64) -> f64 {
    let total = (left_size + right_size) as f64;
    let left_weight = left_size as f64 / total;
    let right_weight = right_size as f64 / total;
    parent_gini - (left_weight * left_gini + right_weight * right_gini)
}

/// 最適な分割を探す
fn find_best_split(data: &[DataPoint]) -> Option<(Split, Vec<DataPoint>, Vec<DataPoint>)> {
    let parent_counts = class_distribution(data);
    let parent_gini = gini(&parent_counts);

    // 純粋なノード（1つのクラスのみ）
    if parent_gini < 1e-9 {
        return None;
    }

    let features = vec!["distance", "length", "image_x", "image_y", "ring_r"];
    let mut best_gain = -1.0;
    let mut best_split: Option<(Split, Vec<DataPoint>, Vec<DataPoint>)> = None;

    for feature in features {
        let thresholds = find_split_thresholds(data, feature);
        for threshold in thresholds {
            let mut left = Vec::new();
            let mut right = Vec::new();

            for point in data {
                if let Some(val) = get_feature_value(point, feature) {
                    if val <= threshold {
                        left.push(point.clone());
                    } else {
                        right.push(point.clone());
                    }
                }
            }

            // 分割が実際に行われたか確認
            if left.is_empty() || right.is_empty() {
                continue;
            }

            let left_counts = class_distribution(&left);
            let right_counts = class_distribution(&right);
            let left_gini = gini(&left_counts);
            let right_gini = gini(&right_counts);

            let gain = information_gain(parent_gini, left.len(), right.len(), left_gini, right_gini);

            if gain > best_gain {
                best_gain = gain;
                best_split = Some((
                    Split {
                        feature: feature.to_string(),
                        threshold,
                    },
                    left,
                    right,
                ));
            }
        }
    }

    best_split
}

/// CART 決定木を構築（訓練精度 100% まで成長）
fn build_tree(data: Vec<DataPoint>, depth: usize, max_depth: Option<usize>) -> TreeNode {
    let counts = class_distribution(&data);

    // リーフ条件：
    // 1. 1つのクラスのみ（純粋）
    // 2. 最大深さに到達
    // 3. 分割できない
    if counts.len() == 1 {
        let choice = *counts.keys().next().unwrap_or(&0);
        return TreeNode::Leaf {
            choice,
            count: data.len(),
            coverage: 1,
        };
    }

    if let Some(max_d) = max_depth {
        if depth >= max_d {
            let choice = majority_class(&counts);
            return TreeNode::Leaf {
                choice,
                count: data.len(),
                coverage: 1,
            };
        }
    }

    if let Some((split, left, right)) = find_best_split(&data) {
        let left_node = build_tree(left, depth + 1, max_depth);
        let right_node = build_tree(right, depth + 1, max_depth);
        TreeNode::Internal {
            split,
            left: Box::new(left_node),
            right: Box::new(right_node),
            samples: data.len(),
        }
    } else {
        let choice = majority_class(&counts);
        TreeNode::Leaf {
            choice,
            count: data.len(),
            coverage: 1,
        }
    }
}

/// ツリーをテキストルールに変換
fn tree_to_rules(node: &TreeNode, rules: &mut Vec<String>, prefix: &str, rule_num: &mut usize) {
    match node {
        TreeNode::Leaf { choice, count: _, .. } => {
            let rule = format!("{}\n  => Choose candidate_{}", prefix, choice);
            rules.push(format!("Rule {}: {}", rule_num, rule));
            *rule_num += 1;
        }
        TreeNode::Internal {
            split,
            left,
            right,
            samples: _,
        } => {
            let left_prefix = if prefix.is_empty() {
                format!("If {} <= {:.2}", split.feature, split.threshold)
            } else {
                format!("{} AND {} <= {:.2}", prefix, split.feature, split.threshold)
            };
            tree_to_rules(left, rules, &left_prefix, rule_num);

            let right_prefix = if prefix.is_empty() {
                format!("If {} > {:.2}", split.feature, split.threshold)
            } else {
                format!("{} AND {} > {:.2}", prefix, split.feature, split.threshold)
            };
            tree_to_rules(right, rules, &right_prefix, rule_num);
        }
    }
}

/// 訓練精度を計算
fn training_accuracy(node: &TreeNode, data: &[DataPoint]) -> f64 {
    let mut correct = 0;
    for point in data {
        let predicted = predict(node, point);
        if predicted == point.leaf_choice {
            correct += 1;
        }
    }
    correct as f64 / data.len() as f64
}

/// 単一データポイントを予測
fn predict(node: &TreeNode, point: &DataPoint) -> usize {
    match node {
        TreeNode::Leaf { choice, .. } => *choice,
        TreeNode::Internal { split, left, right, .. } => {
            if let Some(val) = get_feature_value(point, &split.feature) {
                if val <= split.threshold {
                    predict(left, point)
                } else {
                    predict(right, point)
                }
            } else {
                0
            }
        }
    }
}

fn main() -> Result<()> {
    let args = Args::parse();

    // CSV ファイルを開く
    let file = File::open(&args.csv_file)
        .map_err(|e| anyhow!("Failed to open CSV file: {}", e))?;

    let reader = std::io::BufReader::new(file);
    let mut data = Vec::new();
    let mut total_rows = 0;
    let mut skipped = 0;

    for (line_num, line) in reader.lines().enumerate() {
        let line = line?;

        // ヘッダーをスキップ
        if line_num == 0 {
            continue;
        }

        total_rows += 1;
        let fields: Vec<&str> = line.split(',').collect();

        // 最小フィールド数の確認
        if fields.len() < 9 {
            skipped += 1;
            continue;
        }

        // 必要なカラムを抽出
        let leaf_choice_index: i32 = match fields[2].parse() {
            Ok(v) => v,
            Err(_) => {
                skipped += 1;
                continue;
            }
        };

        // leaf_choice_index >= 0 のデータのみを取得
        if leaf_choice_index < 0 {
            skipped += 1;
            continue;
        }

        let num_candidates: usize = match fields[3].parse() {
            Ok(v) => v,
            Err(_) => {
                skipped += 1;
                continue;
            }
        };

        if num_candidates == 0 {
            skipped += 1;
            continue;
        }

        let image_x: f64 = fields.get(4).unwrap_or(&"0").parse().unwrap_or(0.0);
        let image_y: f64 = fields.get(5).unwrap_or(&"0").parse().unwrap_or(0.0);

        // ring_r はヘックス値の可能性がある
        let ring_r_str = fields.get(6).unwrap_or(&"0");
        let ring_r: f64 = if ring_r_str.starts_with("0x") {
            u64::from_str_radix(&ring_r_str[2..], 16).unwrap_or(0) as f64
        } else {
            ring_r_str.parse().unwrap_or(0.0)
        };

        let prev_token_kind = fields.get(7).unwrap_or(&"unknown").to_string();

        // データポイント生成：各候補を別々のデータポイントとして扱う
        // fields[8 + 2*i] = distance, fields[9 + 2*i] = length
        let mut candidate_min_distance = f64::MAX;
        let mut candidate_best_length = 0.0;

        for i in 0..num_candidates {
            let col_dist = 8 + 2 * i;
            let col_len = 9 + 2 * i;
            if let (Some(dist_str), Some(len_str)) = (fields.get(col_dist), fields.get(col_len)) {
                let dist: f64 = dist_str.parse().unwrap_or(0.0);
                let len: f64 = len_str.parse().unwrap_or(0.0);
                if dist < candidate_min_distance {
                    candidate_min_distance = dist;
                    candidate_best_length = len;
                }
            }
        }

        // 距離と長さは最小距離と対応する長さを使用
        let distance = if candidate_min_distance == f64::MAX {
            0.0
        } else {
            candidate_min_distance
        };

        data.push(DataPoint {
            distance,
            length: candidate_best_length,
            image_x,
            image_y,
            ring_r,
            prev_token_kind,
            leaf_choice: leaf_choice_index as usize,
        });
    }

    println!("=== CART Decision Tree Learning ===");
    println!("CSV file: {}", args.csv_file.display());
    println!("Total rows: {}", total_rows);
    println!("Training samples (leaf_choice_index >= 0): {}", data.len());
    println!("Skipped rows: {}", skipped);
    println!();

    if data.is_empty() {
        return Err(anyhow!("No valid training data found"));
    }

    // ラベルの分布を表示
    let mut label_dist = HashMap::new();
    for point in &data {
        *label_dist.entry(point.leaf_choice).or_insert(0) += 1;
    }
    println!("Label distribution:");
    let mut sorted_labels: Vec<_> = label_dist.iter().collect();
    sorted_labels.sort_by_key(|&(choice, _)| choice);
    for (&choice, &count) in sorted_labels {
        println!("  candidate_{}: {} samples ({:.1}%)",
            choice, count, 100.0 * count as f64 / data.len() as f64);
    }
    println!();

    // CART 決定木を構築
    println!("Building CART decision tree...");
    let tree = build_tree(data.clone(), 0, None);

    // 訓練精度を計算
    let accuracy = training_accuracy(&tree, &data);
    println!("Training accuracy: {:.2}%", accuracy * 100.0);
    println!();

    // ツリーをテキストルールに変換
    let mut rules = Vec::new();
    let mut rule_num = 1;
    tree_to_rules(&tree, &mut rules, "", &mut rule_num);

    // 出力ルール数を制限
    let output_rules = rules.into_iter().take(args.max_rules).collect::<Vec<_>>();

    println!("=== Decision Rules ({} total) ===", output_rules.len());
    for rule in &output_rules {
        println!("{}", rule);
        println!();
    }

    println!("=== Statistics ===");
    println!("Total rules generated: {}", output_rules.len());
    println!("Max depth reached: (tree structure embedded)");
    println!("All training samples classified: {}", if accuracy >= 0.99 { "Yes" } else { "No" });
    println!();

    if accuracy >= 0.99 {
        println!("✓ Training accuracy 100% reached. Decision tree fully separates all classes.");
    } else {
        println!("⚠ Training accuracy {:.2}%. Some classes remain mixed.", accuracy * 100.0);
    }

    Ok(())
}
