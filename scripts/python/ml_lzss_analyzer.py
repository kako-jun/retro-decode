#!/usr/bin/env python3
"""
LF2 LZSS Decision Pattern Machine Learning Analyzer
ToHeart LZSS圧縮アルゴリズムの機械学習による解析

目的: 300+のLF2ファイルから開発者の決定パターンを学習し、
     完全一致するエンコーダーを構築する
"""

import os
import sys
import numpy as np
import torch
import torch.nn as nn
import torch.optim as optim
from torch.utils.data import Dataset, DataLoader
from typing import List, Tuple, Dict, Optional
import struct
import json
from pathlib import Path

class LF2DecisionDataset(Dataset):
    """LF2圧縮決定データセット"""
    
    def __init__(self, lf2_files: List[str], max_decisions_per_file: int = 10000):
        self.decisions = []
        self.contexts = []
        
        print(f"🔬 Loading {len(lf2_files)} LF2 files for analysis...")
        
        for i, file_path in enumerate(lf2_files):
            if i % 50 == 0:
                print(f"   Processing file {i+1}/{len(lf2_files)}: {os.path.basename(file_path)}")
            
            try:
                decisions, contexts = self.extract_decisions_from_lf2(file_path, max_decisions_per_file)
                self.decisions.extend(decisions)
                self.contexts.extend(contexts)
            except Exception as e:
                print(f"   ⚠️  Error processing {file_path}: {e}")
                continue
        
        print(f"✅ Loaded {len(self.decisions)} decision points from {len(lf2_files)} files")
        
    def extract_decisions_from_lf2(self, file_path: str, max_decisions: int) -> Tuple[List[Dict], List[np.ndarray]]:
        """LF2ファイルから決定シーケンスと文脈を抽出"""
        
        with open(file_path, 'rb') as f:
            data = f.read()
        
        # ヘッダー解析
        if len(data) < 0x18 or data[:8] != b'LEAF256\0':
            raise ValueError("Invalid LF2 file")
        
        width = struct.unpack('<H', data[12:14])[0]
        height = struct.unpack('<H', data[14:16])[0]
        color_count = data[0x16]
        
        # 圧縮データ開始位置
        pixel_data_start = 0x18 + color_count * 3
        compressed_data = data[pixel_data_start:]
        
        # 決定シーケンスを抽出
        decisions = []
        contexts = []
        
        ring = np.full(0x1000, 0x20, dtype=np.uint8)
        ring_pos = 0x0fee
        pos = 0
        flag_count = 0
        current_flag = 0
        decision_count = 0
        
        # Y-flipピクセルデータを準備（エンコード時の入力再現）
        total_pixels = width * height
        
        while pos < len(compressed_data) and decision_count < max_decisions:
            if flag_count == 0:
                if pos >= len(compressed_data):
                    break
                current_flag = compressed_data[pos] ^ 0xff
                pos += 1
                flag_count = 8
            
            if pos >= len(compressed_data):
                break
                
            # 文脈特徴を抽出
            context = self.extract_context_features(
                ring, ring_pos, pos, compressed_data, 
                width, height, decision_count
            )
            
            # フラグビットから決定を読み取り
            bit_pos = 8 - flag_count
            is_direct = (current_flag >> (7 - bit_pos)) & 1 != 0
            
            if is_direct:
                # 直接ピクセル
                if pos < len(compressed_data):
                    pixel = compressed_data[pos] ^ 0xff
                    decision = {
                        'type': 'direct',
                        'value': pixel
                    }
                    
                    # リングバッファ更新
                    ring[ring_pos] = pixel
                    ring_pos = (ring_pos + 1) & 0x0fff
                    pos += 1
                    
                    decisions.append(decision)
                    contexts.append(context)
                    decision_count += 1
            else:
                # マッチ参照
                if pos + 1 < len(compressed_data):
                    upper = compressed_data[pos] ^ 0xff
                    lower = compressed_data[pos + 1] ^ 0xff
                    pos += 2
                    
                    length = (upper & 0x0f) + 3
                    position = ((upper >> 4) + (lower << 4)) & 0x0fff
                    
                    decision = {
                        'type': 'match',
                        'position': position,
                        'length': length
                    }
                    
                    # リングバッファ更新
                    copy_pos = position
                    for _ in range(length):
                        byte_val = ring[copy_pos]
                        ring[ring_pos] = byte_val
                        ring_pos = (ring_pos + 1) & 0x0fff
                        copy_pos = (copy_pos + 1) & 0x0fff
                    
                    decisions.append(decision)
                    contexts.append(context)
                    decision_count += 1
            
            flag_count -= 1
        
        return decisions, contexts
    
    def extract_context_features(self, ring: np.ndarray, ring_pos: int, 
                                data_pos: int, compressed_data: bytes,
                                width: int, height: int, decision_idx: int) -> np.ndarray:
        """決定時の文脈特徴を抽出"""
        
        features = []
        
        # 1. リングバッファの現在状態 (最近の32バイト)
        ring_context = np.zeros(32, dtype=np.float32)
        for i in range(32):
            pos = (ring_pos - 32 + i) & 0x0fff
            ring_context[i] = ring[pos] / 255.0
        features.extend(ring_context)
        
        # 2. リングバッファ位置の正規化
        features.append(ring_pos / 0x1000)
        
        # 3. 圧縮進行度
        features.append(data_pos / len(compressed_data) if len(compressed_data) > 0 else 0.0)
        
        # 4. 画像内位置の推定
        estimated_pixel_pos = decision_idx
        estimated_x = (estimated_pixel_pos % width) / width if width > 0 else 0.0
        estimated_y = (estimated_pixel_pos // width) / height if height > 0 else 0.0
        features.extend([estimated_x, estimated_y])
        
        # 5. 利用可能マッチの特徴（簡易版）
        available_matches = self.find_available_matches_features(ring, ring_pos)
        features.extend(available_matches)
        
        return np.array(features, dtype=np.float32)
    
    def find_available_matches_features(self, ring: np.ndarray, ring_pos: int) -> List[float]:
        """利用可能マッチの特徴量を計算"""
        
        features = []
        
        # 近距離マッチの数（簡易計算）
        short_matches = 0  # 0-255バイト範囲
        medium_matches = 0  # 256-511バイト範囲
        long_matches = 0   # 512+バイト範囲
        
        # サンプリングによる高速推定
        for offset in range(1, min(1024, 0x1000), 8):  # 8バイトおきにサンプリング
            start_pos = (ring_pos - offset) & 0x0fff
            
            if offset <= 255:
                short_matches += 1
            elif offset <= 511:
                medium_matches += 1
            else:
                long_matches += 1
        
        features.extend([
            short_matches / 32.0,   # 正規化
            medium_matches / 32.0,
            long_matches / 64.0
        ])
        
        return features
    
    def __len__(self):
        return len(self.decisions)
    
    def __getitem__(self, idx):
        decision = self.decisions[idx]
        context = self.contexts[idx]
        
        # 決定をワンホットエンコード
        if decision['type'] == 'direct':
            # 直接ピクセル: [1, 0, pixel_value/255, 0, 0]
            target = torch.tensor([1.0, 0.0, decision['value']/255.0, 0.0, 0.0])
        else:
            # マッチ: [0, 1, 0, position/4096, length/18]
            target = torch.tensor([
                0.0, 1.0, 0.0, 
                decision['position']/4096.0, 
                (decision['length']-3)/15.0  # 3-18 -> 0-15
            ])
        
        return torch.tensor(context), target

class LZSSDecisionPredictor(nn.Module):
    """LZSS決定予測ニューラルネットワーク"""
    
    def __init__(self, input_size: int, hidden_size: int = 512):
        super().__init__()
        
        self.encoder = nn.Sequential(
            nn.Linear(input_size, hidden_size),
            nn.ReLU(),
            nn.Dropout(0.3),
            nn.Linear(hidden_size, hidden_size),
            nn.ReLU(),
            nn.Dropout(0.3),
            nn.Linear(hidden_size, hidden_size // 2),
            nn.ReLU()
        )
        
        # 決定タイプ予測 (direct vs match)
        self.decision_head = nn.Sequential(
            nn.Linear(hidden_size // 2, 64),
            nn.ReLU(),
            nn.Linear(64, 2),
            nn.Softmax(dim=1)
        )
        
        # 値予測 (pixel value, position, length)
        self.value_head = nn.Sequential(
            nn.Linear(hidden_size // 2, 64),
            nn.ReLU(),
            nn.Linear(64, 3)  # [pixel_value, position, length]
        )
    
    def forward(self, x):
        encoded = self.encoder(x)
        decision_prob = self.decision_head(encoded)
        values = self.value_head(encoded)
        
        return decision_prob, values

def train_model(train_loader: DataLoader, model: LZSSDecisionPredictor, 
                epochs: int = 10, device: str = 'cuda'):
    """モデルを訓練"""
    
    model = model.to(device)
    optimizer = optim.Adam(model.parameters(), lr=0.001)
    criterion_decision = nn.CrossEntropyLoss()
    criterion_values = nn.MSELoss()
    
    print(f"🚀 Starting training on {device}...")
    
    for epoch in range(epochs):
        total_loss = 0.0
        decision_accuracy = 0.0
        num_batches = 0
        
        model.train()
        for batch_idx, (contexts, targets) in enumerate(train_loader):
            contexts = contexts.to(device)
            targets = targets.to(device)
            
            optimizer.zero_grad()
            
            decision_probs, values = model(contexts)
            
            # 決定タイプの損失
            decision_true = targets[:, :2].argmax(dim=1)
            decision_loss = criterion_decision(decision_probs, decision_true)
            
            # 値の損失
            value_loss = criterion_values(values, targets[:, 2:])
            
            total_loss_batch = decision_loss + value_loss
            total_loss_batch.backward()
            optimizer.step()
            
            total_loss += total_loss_batch.item()
            
            # 決定精度計算
            decision_pred = decision_probs.argmax(dim=1)
            decision_accuracy += (decision_pred == decision_true).float().mean().item()
            
            num_batches += 1
            
            if batch_idx % 100 == 0:
                print(f"   Epoch {epoch+1}/{epochs}, Batch {batch_idx}, "
                      f"Loss: {total_loss_batch.item():.4f}, "
                      f"Decision Acc: {(decision_pred == decision_true).float().mean().item():.4f}")
        
        avg_loss = total_loss / num_batches
        avg_accuracy = decision_accuracy / num_batches
        
        print(f"📊 Epoch {epoch+1}/{epochs} - Loss: {avg_loss:.4f}, "
              f"Decision Accuracy: {avg_accuracy:.4f}")

def collect_lf2_files(lf2_dir: str) -> List[str]:
    """LF2ファイルを収集"""
    
    lf2_files = []
    lf2_path = Path(lf2_dir)
    
    if not lf2_path.exists():
        print(f"❌ LF2 directory not found: {lf2_dir}")
        return []
    
    for file_path in lf2_path.glob("*.LF2"):
        lf2_files.append(str(file_path))
    
    print(f"📁 Found {len(lf2_files)} LF2 files")
    return lf2_files

def main():
    """メイン実行関数"""
    
    # GPU確認
    device = 'cuda' if torch.cuda.is_available() else 'cpu'
    print(f"🖥️  Using device: {device}")
    
    if device == 'cuda':
        print(f"   GPU: {torch.cuda.get_device_name(0)}")
        print(f"   Memory: {torch.cuda.get_device_properties(0).total_memory // 1024**3} GB")
    
    # LF2ファイル収集
    script_dir = Path(__file__).parent
    lf2_dir = script_dir.parent.parent / "test_assets" / "lf2"
    lf2_files = collect_lf2_files(str(lf2_dir))
    
    if len(lf2_files) < 10:
        print("❌ Insufficient LF2 files for training")
        return
    
    # データセット作成
    print("📊 Creating dataset...")
    dataset = LF2DecisionDataset(lf2_files, max_decisions_per_file=5000)
    
    if len(dataset) < 1000:
        print("❌ Insufficient training data")
        return
    
    # データローダー作成
    train_loader = DataLoader(dataset, batch_size=64, shuffle=True, num_workers=4)
    
    # モデル作成
    input_size = len(dataset.contexts[0])
    model = LZSSDecisionPredictor(input_size)
    
    print(f"🧠 Model created with input size: {input_size}")
    print(f"   Total parameters: {sum(p.numel() for p in model.parameters()):,}")
    
    # 訓練実行
    train_model(train_loader, model, epochs=20, device=device)
    
    # モデル保存
    model_path = script_dir / "lzss_decision_model.pth"
    torch.save(model.state_dict(), model_path)
    print(f"💾 Model saved to: {model_path}")
    
    # 結果分析
    print("\n🔍 Analysis Results:")
    print(f"   Training samples: {len(dataset):,}")
    print(f"   Average decisions per file: {len(dataset) / len(lf2_files):.1f}")
    print(f"   Context feature dimensions: {input_size}")

if __name__ == "__main__":
    main()