from PIL import Image
from collections import Counter

def get_dominant_colors(image_path, num_colors=3):
    try:
        image = Image.open(image_path)
        image = image.resize((50, 50))
        # Convert to RGB if not
        if image.mode != 'RGB':
            image = image.convert('RGB')
        
        pixels = list(image.getdata())
        counts = Counter(pixels)
        most_common = counts.most_common(num_colors)
        
        hex_colors = []
        for color, count in most_common:
            hex_color = '#{:02x}{:02x}{:02x}'.format(*color)
            hex_colors.append((hex_color, count))
            
        return hex_colors
    except Exception as e:
        print(f"Error: {e}")
        return []

colors = get_dominant_colors('docs/media/logo.png', 5)
print("Dominant Colors:")
for hex_c, count in colors:
    print(f"{hex_c} ({count})")
