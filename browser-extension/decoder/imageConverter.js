/**
 * Image Converter for Browser Extension
 * Converts raw pixel data to PNG data URL
 */

function convertToDataURL(imageData) {
  const { width, height, data } = imageData;

  // Create canvas
  const canvas = document.createElement('canvas');
  canvas.width = width;
  canvas.height = height;
  const ctx = canvas.getContext('2d');

  // Create ImageData and put pixels
  const imgData = ctx.createImageData(width, height);
  imgData.data.set(data);
  ctx.putImageData(imgData, 0, 0);

  // Convert to data URL (PNG)
  return canvas.toDataURL('image/png');
}
