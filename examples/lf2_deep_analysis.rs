//! LF2ファイルの詳細解析 - オリジナル圧縮アルゴリズムの完全理解

use retro_decode::formats::toheart::Lf2Image;
use anyhow::Result;
use std::fs;

fn main() -> Result<()> {
    println!("🔬 LF2 Deep Analysis - Original Compression Algorithm Study");
    println!("============================================================");
    
    // オリジナルLF2ファイルの圧縮データを直接解析
    let original_data = fs::read("test_assets/lf2/C170A.LF2")?;
    
    println!("📊 Original LF2 File Analysis:");
    println!("   File size: {} bytes", original_data.len());
    
    // ヘッダー解析
    analyze_header(&original_data)?;
    
    // 圧縮データ部分の開始位置を特定
    let color_count = original_data[0x16];
    let pixel_data_start = 0x18 + (color_count as usize) * 3;
    
    println!("\n🗜️  Compressed Pixel Data Analysis:");
    println!("   Start offset: 0x{:x} ({})", pixel_data_start, pixel_data_start);
    println!("   Compressed size: {} bytes", original_data.len() - pixel_data_start);
    
    // 圧縮データの詳細解析
    analyze_compressed_data(&original_data[pixel_data_start..])?;
    
    // 我々の実装でデコード
    let decoded_image = Lf2Image::open("test_assets/lf2/C170A.LF2")?;
    println!("\n📤 Our Decoding Result:");
    println!("   Pixel count: {}", decoded_image.pixels.len());
    println!("   Expected pixels: {}", decoded_image.width as usize * decoded_image.height as usize);
    
    // 我々の実装でエンコード（Level 0）
    let reencoded_data = decoded_image.to_lf2_bytes()?;
    println!("\n🔄 Our Re-encoding (Level 0):");
    println!("   File size: {} bytes", reencoded_data.len());
    println!("   Size ratio: {:.1}%", (reencoded_data.len() as f64 / original_data.len() as f64) * 100.0);
    
    // 圧縮データ部分の比較
    let reencoded_pixel_start = 0x18 + (color_count as usize) * 3;
    compare_compression_data(
        &original_data[pixel_data_start..],
        &reencoded_data[reencoded_pixel_start..]
    )?;
    
    Ok(())
}

fn analyze_header(data: &[u8]) -> Result<()> {
    println!("\n📋 Header Analysis:");
    println!("   Magic: {:?}", &data[0..8]);
    println!("   X offset: {}", u16::from_le_bytes([data[8], data[9]]));
    println!("   Y offset: {}", u16::from_le_bytes([data[10], data[11]]));
    println!("   Width: {}", u16::from_le_bytes([data[12], data[13]]));
    println!("   Height: {}", u16::from_le_bytes([data[14], data[15]]));
    println!("   Transparent color: {}", data[0x12]);
    println!("   Color count: {}", data[0x16]);
    
    Ok(())
}

fn analyze_compressed_data(compressed: &[u8]) -> Result<()> {
    println!("   First 32 bytes: {:02x?}", &compressed[..32.min(compressed.len())]);
    
    // フラグバイトの分布を解析
    let mut flag_positions = Vec::new();
    let mut pos = 0;
    let mut flag_count = 0;
    
    while pos < compressed.len() {
        // フラグバイトを読む
        if flag_count == 0 {
            flag_positions.push(pos);
            if pos < compressed.len() {
                let flag = compressed[pos] ^ 0xff;
                pos += 1;
                flag_count = 8;
                
                // フラグバイトの詳細解析
                if flag_positions.len() <= 10 { // 最初の10個だけ表示
                    println!("   Flag byte at {}: 0x{:02x} = {:08b}", 
                        flag_positions.last().unwrap(), flag, flag);
                }
            }
        }
        
        if pos >= compressed.len() { break; }
        
        // フラグビットに基づいて処理
        let flag = compressed[*flag_positions.last().unwrap()] ^ 0xff;
        let bit_pos = 8 - flag_count;
        let is_direct = (flag >> (7 - bit_pos)) & 1 != 0;
        
        if is_direct {
            // 直接ピクセル
            pos += 1;
        } else {
            // リングバッファ参照
            if pos + 1 < compressed.len() {
                let upper = compressed[pos] ^ 0xff;
                let lower = compressed[pos + 1] ^ 0xff;
                let length = (upper & 0x0f) + 3;
                let position = ((upper >> 4) as usize) + ((lower as usize) << 4);
                
                if flag_positions.len() <= 3 { // 最初の3個のフラグバイトの詳細のみ
                    println!("   Match at {}: len={}, pos=0x{:03x}", pos, length, position);
                }
                
                pos += 2;
            } else {
                break;
            }
        }
        
        flag_count -= 1;
    }
    
    println!("   Total flag bytes found: {}", flag_positions.len());
    
    Ok(())
}

fn compare_compression_data(original: &[u8], reencoded: &[u8]) -> Result<()> {
    println!("\n🔍 Compression Data Comparison:");
    println!("   Original size: {} bytes", original.len());
    println!("   Re-encoded size: {} bytes", reencoded.len());
    println!("   Size difference: {} bytes", reencoded.len() as i32 - original.len() as i32);
    
    // 最初の100バイトを16進ダンプで比較
    let compare_len = 100.min(original.len()).min(reencoded.len());
    
    println!("\n📄 First {} bytes comparison:", compare_len);
    println!("   Pos: Original    Re-encoded   Diff");
    println!("   ---: --------    ----------   ----");
    
    for i in 0..compare_len {
        let orig = original[i];
        let reenc = reencoded[i];
        let diff_char = if orig == reenc { " " } else { "!" };
        
        if i < 50 || orig != reenc { // 最初の50バイトまたは差異がある箇所のみ表示
            println!("   {:3}: {:02x}       {:02x}       {}", i, orig, reenc, diff_char);
        }
        
        if i == 49 && compare_len > 50 {
            println!("   ... (showing only first 50 bytes and differences)");
        }
    }
    
    // 差異の統計
    let mut diff_count = 0;
    let min_len = original.len().min(reencoded.len());
    for i in 0..min_len {
        if original[i] != reencoded[i] {
            diff_count += 1;
        }
    }
    
    println!("\n📊 Compression Difference Statistics:");
    println!("   Byte differences: {} / {} ({:.2}%)", 
        diff_count, min_len, (diff_count as f64 / min_len as f64) * 100.0);
    
    Ok(())
}