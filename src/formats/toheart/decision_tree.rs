//! Phase 3 推論用の決定木ローダ・走査。
//!
//! 学習時 (`train_decision_tree`) と同じ `TreeNode` 形式を bincode で読み込み、
//! 候補集合の中で Leaf エンコーダが選ぶインデックスを推論する。
//!
//! 学習側との約束:
//! - 特徴量: image_x, length (= 最小距離候補の length), image_y, ring_r
//! - 候補集合は `enumerate_match_candidates_with_writeback` の出力順
//!   （pos 昇順 → len 昇順）。インデックスはこの並びを参照。
//!
//! ロード戦略:
//! - 環境変数 `RETRO_DECODE_TREE_PATH` が指定されていればそのパスから読む
//! - そうでなければ `models/lf2_decision_tree.bin`（リポルートからの相対）
//! - どちらも失敗したらエラー

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Split {
    pub feature: String,
    pub threshold: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TreeNode {
    Leaf {
        choice: usize,
        count: usize,
        coverage: usize,
    },
    Internal {
        split: Split,
        left: Box<TreeNode>,
        right: Box<TreeNode>,
        samples: usize,
    },
}

impl TreeNode {
    /// 4 つの特徴量から候補リスト中のインデックスを予測する。
    pub fn predict(&self, image_x: f64, length: f64, image_y: f64, ring_r: f64) -> usize {
        let mut node = self;
        loop {
            match node {
                TreeNode::Leaf { choice, .. } => return *choice,
                TreeNode::Internal {
                    split,
                    left,
                    right,
                    ..
                } => {
                    let val = match split.feature.as_str() {
                        "image_x" => image_x,
                        "length" => length,
                        "image_y" => image_y,
                        "ring_r" => ring_r,
                        // 学習側で削除済みの "distance" などが残っていても
                        // 安全に左へ倒す（決定木は左右対称ではないが、
                        // 未知特徴量は致命傷になり得るのでここで気付ける）
                        _ => f64::NEG_INFINITY,
                    };
                    if val <= split.threshold {
                        node = left;
                    } else {
                        node = right;
                    }
                }
            }
        }
    }
}

fn resolve_tree_path() -> PathBuf {
    if let Ok(p) = std::env::var("RETRO_DECODE_TREE_PATH") {
        return PathBuf::from(p);
    }
    // CARGO_MANIFEST_DIR はビルド時定数。実行時は必ず存在する
    // (cargo build/run 経由) ので、これを基準にした相対パスを使う。
    let manifest = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest).join("models/lf2_decision_tree.bin")
}

fn load_tree() -> Result<TreeNode> {
    let path = resolve_tree_path();
    let bytes = fs::read(&path)
        .map_err(|e| anyhow!("decision tree load failed at {}: {}", path.display(), e))?;
    let tree: TreeNode = bincode::deserialize(&bytes)
        .map_err(|e| anyhow!("decision tree deserialize failed: {}", e))?;
    Ok(tree)
}

static TREE: OnceLock<Result<TreeNode, String>> = OnceLock::new();

/// グローバル決定木へのアクセサ。最初の呼び出し時にロードしてキャッシュする。
pub fn global_tree() -> Result<&'static TreeNode> {
    let cell = TREE.get_or_init(|| load_tree().map_err(|e| e.to_string()));
    match cell {
        Ok(t) => Ok(t),
        Err(e) => Err(anyhow!("{}", e)),
    }
}
