"""Build GPUI's static DM Sans faces from the bundled variable fonts.

Run with Python's fonttools package installed. CoreText's GPUI family matching
selects concrete faces; registering a variable face alone loses weight choices.
"""
from pathlib import Path
from fontTools.ttLib import TTFont
from fontTools.varLib.instancer import instantiateVariableFont

root = Path(__file__).resolve().parent.parent / '.dependencies/app-fonts/dm-sans'
output = root / 'static'
output.mkdir(exist_ok=True)
for source, italic in [('normal', False), ('italic', True)]:
    for weight, style in [(400, 'Regular'), (500, 'Medium'), (600, 'SemiBold'), (700, 'Bold')]:
        font = instantiateVariableFont(TTFont(root / f'{source}.ttf'), {'wght': weight}, inplace=True)
        subfamily = ('Italic' if style == 'Regular' else f'{style} Italic') if italic else style
        postscript = 'DMSans-' + subfamily.replace(' ', '')
        names = {1: 'DM Sans', 2: subfamily, 4: f'DM Sans {subfamily}', 6: postscript, 16: 'DM Sans', 17: subfamily}
        for name_id, value in names.items():
            font['name'].setName(value, name_id, 3, 1, 0x409)
            font['name'].setName(value, name_id, 1, 0, 0)
        font.save(output / f'{weight}{"-italic" if italic else ""}.ttf')
