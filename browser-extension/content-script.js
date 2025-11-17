/**
 * RetroDecode Browser Extension - Content Script
 * Automatically converts LF2/PDT images to PNG
 */

(function() {
  'use strict';

  console.log('[RetroDecode] Extension loaded');

  // Track processed images to avoid duplicate processing
  const processedImages = new WeakSet();

  /**
   * Process a single image element
   */
  async function processImage(img) {
    // Skip if already processed
    if (processedImages.has(img)) {
      return;
    }

    const src = img.src;
    if (!src) return;

    // Check if it's a retro game format
    const isLF2 = src.toLowerCase().endsWith('.lf2');
    const isPDT = src.toLowerCase().endsWith('.pdt');

    if (!isLF2 && !isPDT) {
      return;
    }

    // Mark as processed immediately to prevent duplicate work
    processedImages.add(img);

    console.log(`[RetroDecode] Processing ${isLF2 ? 'LF2' : 'PDT'}: ${src}`);

    try {
      // Add loading indicator
      img.style.filter = 'blur(4px)';
      img.alt = `[Loading ${isLF2 ? 'LF2' : 'PDT'}...] ${img.alt || ''}`;

      // Fetch the file
      const response = await fetch(src);
      if (!response.ok) {
        throw new Error(`HTTP ${response.status}: ${response.statusText}`);
      }

      const arrayBuffer = await response.arrayBuffer();
      const data = new Uint8Array(arrayBuffer);

      // Decode based on format
      let imageData;
      if (isLF2) {
        imageData = decodeLF2(data);
      } else if (isPDT) {
        imageData = decodePDT(data);
      }

      // Convert to PNG data URL
      const dataURL = convertToDataURL(imageData);

      // Replace image source
      img.src = dataURL;
      img.style.filter = '';
      img.alt = img.alt.replace(/^\[Loading (LF2|PDT)...\] /, '');

      console.log(`[RetroDecode] Successfully converted: ${src}`);

    } catch (error) {
      console.error(`[RetroDecode] Failed to convert ${src}:`, error);
      img.style.filter = '';
      img.alt = `[Decode Error] ${img.alt.replace(/^\[Loading (LF2|PDT)...\] /, '')}`;
      img.title = `RetroDecode Error: ${error.message}`;
      // Show error as broken image with red border
      img.style.border = '2px solid red';
    }
  }

  /**
   * Process all matching images in a container
   */
  function processAllImages(container = document) {
    const images = container.querySelectorAll('img[src$=".lf2"], img[src$=".LF2"], img[src$=".pdt"], img[src$=".PDT"]');
    images.forEach(img => processImage(img));
  }

  /**
   * Observe DOM for dynamically added images
   */
  function observeDOM() {
    const observer = new MutationObserver((mutations) => {
      mutations.forEach((mutation) => {
        mutation.addedNodes.forEach((node) => {
          if (node.nodeType === 1) { // Element node
            if (node.tagName === 'IMG') {
              processImage(node);
            } else {
              processAllImages(node);
            }
          }
        });

        // Watch for src attribute changes
        if (mutation.type === 'attributes' && mutation.attributeName === 'src') {
          if (mutation.target.tagName === 'IMG') {
            // Remove from processed set so it can be re-processed
            processedImages.delete(mutation.target);
            processImage(mutation.target);
          }
        }
      });
    });

    observer.observe(document.body, {
      childList: true,
      subtree: true,
      attributes: true,
      attributeFilter: ['src']
    });

    console.log('[RetroDecode] DOM observer started');
  }

  // Initialize on page load
  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', () => {
      processAllImages();
      observeDOM();
    });
  } else {
    processAllImages();
    observeDOM();
  }

})();
