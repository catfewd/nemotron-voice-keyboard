from PIL import Image, ImageDraw

def create_icon():
    # 512x512 canvas
    size = 512
    # Nanobana Yellow (Gold)
    bg_color = (255, 215, 0)
    # Black for the cat
    cat_color = (0, 0, 0)
    # White for the symbol
    symbol_color = (255, 255, 255)

    img = Image.new('RGB', (size, size), bg_color)
    draw = ImageDraw.Draw(img)

    # 1. Draw Cat Head Silhouette
    # Main head circle
    head_center = (256, 300)
    head_radius = 140
    draw.ellipse([head_center[0]-head_radius, head_center[1]-head_radius, 
                  head_center[0]+head_radius, head_center[1]+head_radius], fill=cat_color)

    # Ears
    # Left Ear
    draw.polygon([(140, 220), (180, 120), (240, 180)], fill=cat_color)
    # Right Ear
    draw.polygon([(372, 220), (332, 120), (272, 180)], fill=cat_color)

    # 2. Draw Masonry Square and Compass Symbol (Minimalist)
    # The Compass (V shape)
    comp_width = 8
    draw.line([(256, 220), (180, 360)], fill=symbol_color, width=comp_width)
    draw.line([(256, 220), (332, 360)], fill=symbol_color, width=comp_width)
    draw.ellipse([250, 214, 262, 226], fill=symbol_color) # Top joint

    # The Square (L shape rotated)
    draw.line([(180, 280), (256, 420)], fill=symbol_color, width=comp_width)
    draw.line([(332, 280), (256, 420)], fill=symbol_color, width=comp_width)

    # Save
    img.save('new_icon.png')

if __name__ == "__main__":
    create_icon()
