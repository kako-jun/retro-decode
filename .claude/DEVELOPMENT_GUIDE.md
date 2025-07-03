# 開発ガイド

## 葉鍵ベンチ（パフォーマンス比較システム）

### 概要
RetroDecode独自のベンチマークシステム。異なる技術手法・制約条件での処理速度を比較し、アルゴリズムの教育的理解を深める。

### Python関連オプション
```bash
--numpy          # NumPy使用（デフォルト）
--no-numpy       # 純Python（リスト・ループのみ）
--numba          # Numba JIT最適化
--cupy           # GPU加速NumPy互換
```

### JavaScript/TypeScript関連オプション
```bash
--typed-arrays   # Uint8Array使用（デフォルト）
--no-typed-arrays # 通常のArray使用
--wasm           # WebAssembly版を呼び出し
--worker-threads # Node.js Worker threads使用
```

### Rust関連オプション
```bash
--unsafe         # unsafeポインタ最適化
--safe-only      # safe Rustのみ（境界チェック有）
--simd           # SIMD命令使用
--single-thread  # シングルスレッド強制
```

### アルゴリズム制約オプション
```bash
--no-bulk-io     # 1バイト読み込み強制
--no-bitwise     # ビット演算禁止（除算・剰余使用）
--naive-ring     # リングバッファ最適化なし
--recursive      # 再帰実装（スタック負荷テスト）
```

### メモリ制約オプション
```bash
--memory-limit <MB>  # 使用メモリ上限
--no-cache           # キャッシュ無効化
--streaming          # ストリーミング処理強制
```

### ベンチマーク使用例
```bash
# 基本比較: 最速 vs 最遅
retro-decode --input file.lf2 --lang rust --unsafe --simd
retro-decode --input file.lf2 --lang python --no-numpy --recursive

# 言語間パフォーマンス比較
retro-decode --input file.pdt --lang rust --benchmark
retro-decode --input file.pdt --lang python --numpy --benchmark
retro-decode --input file.pdt --lang typescript --typed-arrays --benchmark

# 制約条件での教育的比較
retro-decode --input archive.pak --no-bulk-io --no-bitwise  # 古典手法
retro-decode --input archive.pak --memory-limit 64         # 制限環境
```

### 葉鍵ベンチ結果出力
- 処理時間詳細（パース・デコード・書き込み別）
- メモリ使用量推移
- CPU使用率
- 最適化手法の効果測定
- 教育的洞察とレコメンデーション

## テスト戦略

### 現在の問題
- 著作物ファイルはローカルのみ（CI/CDで使用不可）
- 合成テストファイルは小さすぎる（4x4, 16x16）
- 実際のゲーム画像との比較ができない

### 解決策
1. **往復テスト**: 既存著作物でバイナリ一致確認
2. **写真変換**: 大きなフリー写真からLF2/PDT生成
3. **段階的検証**: 
   - パレット生成精度
   - LZSS圧縮効率
   - 透過処理正確性
   - 色減少品質

### テスト資産状況

#### ✅ 利用可能（コミット済み）
```
test_assets/generated/
├── transparency_4x4.png      # 透過テスト
├── pattern_16x16.png         # パターンテスト  
├── palette_boundary.png      # 境界条件テスト
└── max_palette_8x8.png       # 最大パレットテスト
```

#### ❌ 不足しているもの
- 大きな画像（512x512以上）のテストケース
- 実際のゲーム画像相当の複雑度
- 往復テスト用の基準ファイル
- パフォーマンステスト用の大容量ファイル

### テスト実行方法
```bash
# 単体テスト
cargo test

# 往復テスト
cargo test roundtrip

# 統合テスト
cargo test integration

# ベンチマークテスト
cargo bench

# 特定フォーマットテスト
cargo test lf2
cargo test pdt
```

## 開発ノート

### パフォーマンス考慮事項
- **メモリ効率**: リングバッファ実装による省メモリ
- **GPU加速**: --gpuオプションによる並列処理
- **並列処理**: --parallelオプション（シングルスレッドとの比較用）
- **ブラウザ最適化**: WebAssemblyによる高性能処理
- **クロスプラットフォーム**: Windows/macOS/Linux対応

### ユーザーエクスペリエンス
- **直感的UI**: タイムラインスクラビング
- **リアルタイム**: プレビュー更新の応答性
- **ダークテーマ**: 技術コンテンツ向けモダン美学
- **アクセシビリティ**: キーボードショートカット対応

### 遵守と倫理
- **ユーザー所有ファイル**: 処理対象の限定
- **教育目的**: 技術解説・歴史保存フォーカス
- **著作権配慮**: コンテンツ配布なし
- **透明性**: 処理内容の可視化

## 開発環境セットアップ

### 必要ツール
```bash
# Rust開発環境
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup update

# 追加コンポーネント
rustup component add rustfmt clippy

# 開発ツール
cargo install cargo-watch
cargo install cargo-expand
```

### 推奨設定
```toml
# .cargo/config.toml
[build]
rustflags = ["-C", "target-cpu=native"]

[env]
RUST_BACKTRACE = "1"
```

### VSCode拡張
- rust-analyzer
- CodeLLDB（デバッグ用）
- Better TOML
- GitLens

## コーディング規約

### Rust Style
```rust
// 関数命名: snake_case
fn decode_lf2_file() -> Result<ImageData, Error> {}

// 型命名: PascalCase  
struct ImageHeader {}
enum CompressionType {}

// 定数: SCREAMING_SNAKE_CASE
const MAX_PALETTE_SIZE: usize = 256;

// エラーハンドリング: Result型必須
pub fn process_file(path: &Path) -> Result<(), ProcessError> {
    let data = std::fs::read(path)?;
    validate_format(&data)?;
    Ok(())
}
```

### ドキュメント
```rust
/// LF2形式の画像ファイルをデコードします
/// 
/// # Arguments
/// * `data` - LF2ファイルのバイナリデータ
/// 
/// # Returns
/// デコードされた画像データまたはエラー
/// 
/// # Example
/// ```
/// let image = decode_lf2(&file_data)?;
/// ```
pub fn decode_lf2(data: &[u8]) -> Result<ImageData, DecodeError> {
    // 実装
}
```

## デバッグ・プロファイリング

### デバッグ手法
```bash
# デバッグビルド実行
cargo run --bin retro-decode -- --input test.lf2 --verbose

# Rust trace有効化
RUST_LOG=debug cargo run

# メモリリーク検査（Linux）
valgrind --tool=memcheck --leak-check=full ./target/debug/retro-decode
```

### プロファイリング
```bash
# CPU プロファイリング（Linux）
perf record --call-graph dwarf cargo run --release
perf report

# ベンチマーク
cargo bench --bench compression_bench
```

## リリース・デプロイ

### ビルド設定
```toml
# Cargo.toml
[profile.release]
lto = true
codegen-units = 1
panic = "abort"
strip = true
```

### クロスコンパイル
```bash
# Windows向けビルド（Linux上）
rustup target add x86_64-pc-windows-gnu
cargo build --release --target x86_64-pc-windows-gnu

# macOS向けビルド
rustup target add x86_64-apple-darwin
cargo build --release --target x86_64-apple-darwin
```

### リリースパッケージ
```bash
# バイナリ + 文書パッケージ作成
./scripts/package_release.sh v0.1.0
```

## トラブルシューティング

### よくある問題
1. **コンパイルエラー**: 依存関係の不整合
2. **パフォーマンス**: リリースビルド使用確認
3. **ファイル読み込みエラー**: パス・権限確認
4. **メモリ不足**: 大容量ファイル処理時

### 解決手順
1. `cargo clean && cargo build`
2. 依存関係更新: `cargo update`
3. Rust更新: `rustup update`
4. 問題の最小再現ケース作成