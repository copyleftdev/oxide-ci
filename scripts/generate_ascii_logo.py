from ascii_magic import AsciiArt
import sys

def main(image_path):
    try:
        my_art = AsciiArt.from_image(image_path)
        my_art.to_terminal(columns=80)
    except Exception as e:
        # Fallback to simple text if fails
        print("\033[38;2;253;68;3mOxide CI\033[0m")

if __name__ == "__main__":
    path = "docs/media/logo.png"
    if len(sys.argv) > 1:
        path = sys.argv[1]
    
    main(path)
