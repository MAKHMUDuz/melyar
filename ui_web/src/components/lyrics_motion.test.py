from pathlib import Path
import sys

overlay_path = Path(sys.argv[1]) if len(sys.argv) > 1 else Path('ui_web/src/components/LyricsOverlay.tsx')
css_path = Path(sys.argv[2]) if len(sys.argv) > 2 else Path('ui_web/src/glass.css')
source = overlay_path.read_text(encoding='utf-8')
css = css_path.read_text(encoding='utf-8')

assert 'const motion = getMotionTokens();' in source, 'LyricsOverlay must select reduced-motion-aware tokens'
assert 'transition: motion.transition.overlay' in source, 'lyrics and controls must use the overlay motion token'
assert source.count('transition: motion.transition.overlay') == 2, 'lyrics and controls must both use the overlay motion token'
assert 'lyrics-overlay' in source, 'lyrics view must be scoped for reduced-motion suppression'
assert '.lyrics-overlay *::after' in css and 'animation: none !important;' in css and 'transition: none !important;' in css, 'all lyrics animations and transitions must stop for reduced motion'
assert 'transition-all duration-300' not in source, 'lyrics still use an unconditional CSS transition'
assert 'transition-opacity duration-300' not in source, 'controls still use an unconditional CSS transition'
