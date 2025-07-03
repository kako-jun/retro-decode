//! Debug utility to analyze specific pixel differences in detail

use retro_decode::formats::toheart::Lf2Image;
use anyhow::Result;

fn main() -> Result<()> {
    println!("🔍 LF2 Pixel Difference Debug Analysis");
    println!("=====================================");
    
    // Load both images
    let original_lf2 = Lf2Image::open("test_assets/lf2/C170A.LF2")?;
    let reencoded_lf2 = Lf2Image::open("test_assets/generated/roundtrip_test.lf2")?;
    
    println!("📊 Basic Info:");
    println!("   Original pixels: {}", original_lf2.pixels.len());
    println!("   Reencoded pixels: {}", reencoded_lf2.pixels.len());
    println!("   Image size: {}x{}", original_lf2.width, original_lf2.height);
    
    // Find all differences
    let mut differences = Vec::new();
    for (i, (orig, reenc)) in original_lf2.pixels.iter().zip(reencoded_lf2.pixels.iter()).enumerate() {
        if orig != reenc {
            let x = i % (original_lf2.width as usize);
            let y = i / (original_lf2.width as usize);
            differences.push((i, x, y, *orig, *reenc));
        }
    }
    
    println!("\n🔍 Found {} pixel differences", differences.len());
    
    if differences.is_empty() {
        println!("✅ No differences found!");
        return Ok(());
    }
    
    // Analyze first 20 differences in detail
    println!("\n📍 First 20 differences:");
    for (i, (pixel_idx, x, y, orig, reenc)) in differences.iter().take(20).enumerate() {
        println!("   {}. Pixel[{}] at ({},{}) = {} → {} (Δ={})", 
            i+1, pixel_idx, x, y, orig, reenc, *reenc as i16 - *orig as i16);
    }
    
    // Check for patterns in differences
    println!("\n🔎 Pattern Analysis:");
    
    // Value frequency analysis
    let mut orig_values = std::collections::HashMap::new();
    let mut reenc_values = std::collections::HashMap::new();
    
    for (_, _, _, orig, reenc) in &differences {
        *orig_values.entry(*orig).or_insert(0) += 1;
        *reenc_values.entry(*reenc).or_insert(0) += 1;
    }
    
    println!("   Original values in differences: {:?}", orig_values);
    println!("   Reencoded values in differences: {:?}", reenc_values);
    
    // Check spatial clustering
    differences.sort_by_key(|(idx, _, _, _, _)| *idx);
    
    println!("\n🗺️  Spatial clustering check:");
    let mut clusters = Vec::new();
    let mut current_cluster = Vec::new();
    
    for i in 0..differences.len() {
        let (pixel_idx, x, y, orig, reenc) = differences[i];
        
        if current_cluster.is_empty() {
            current_cluster.push((pixel_idx, x, y, orig, reenc));
        } else {
            let last_idx = current_cluster.last().unwrap().0;
            if pixel_idx - last_idx <= 10 {  // Within 10 pixels
                current_cluster.push((pixel_idx, x, y, orig, reenc));
            } else {
                if current_cluster.len() > 1 {
                    clusters.push(current_cluster.clone());
                }
                current_cluster.clear();
                current_cluster.push((pixel_idx, x, y, orig, reenc));
            }
        }
    }
    
    if current_cluster.len() > 1 {
        clusters.push(current_cluster);
    }
    
    println!("   Found {} clusters of differences", clusters.len());
    for (i, cluster) in clusters.iter().take(5).enumerate() {
        println!("   Cluster {}: {} pixels from {} to {}", 
            i+1, cluster.len(), cluster[0].0, cluster.last().unwrap().0);
    }
    
    // Check if differences correlate with specific values
    println!("\n🎯 Value transition analysis:");
    let mut transitions = std::collections::HashMap::new();
    for (_, _, _, orig, reenc) in &differences {
        let transition = (*orig, *reenc);
        *transitions.entry(transition).or_insert(0) += 1;
    }
    
    println!("   Top 10 value transitions:");
    let mut sorted_transitions: Vec<_> = transitions.iter().collect();
    sorted_transitions.sort_by_key(|(_, count)| -(**count as i32));
    
    for (i, ((orig, reenc), count)) in sorted_transitions.iter().take(10).enumerate() {
        println!("   {}. {} → {} ({} times)", i+1, orig, reenc, count);
    }
    
    Ok(())
}