"""Rebuild Dibby's SVG motion variants, native atlas, contract, and self-contained preview."""
from pathlib import Path
import asyncio
import hashlib
import io
import json
from xml.etree import ElementTree as ET

from PIL import Image
from playwright.async_api import async_playwright

ROOT = Path(__file__).resolve().parents[1]
ASSETS = ROOT / 'assets' / 'mascot'
SVG = (ASSETS / 'dibby.svg').read_text(encoding='utf-8')
RIG = (ROOT / 'tools' / 'dibby-rig.js').read_text(encoding='utf-8')
STATES = {'neutral': (0, 1, 0), 'idle': (1, 36, 2400), 'hover': (37, 36, 600),
          'talking': (73, 36, 1200), 'thinking': (109, 36, 3000), 'working': (145, 36, 3000)}
TILE, COLS, ROWS = 160, 12, 16

CSS = '''
svg { overflow: visible; }
#float, #body-rock, #orbit-back-motion, #orbit-front-motion, #eyes-motion, #mouth-talk, #thought-cloud, #laptop {
  transform-box: view-box; transform-origin: 630px 640px;
}
#eyes-motion { transform-origin: 600px 535px; }
#mouth-talk { transform-origin: 655px 645px; }
#laptop { transform-origin: 1050px 990px; }
@keyframes idle-float { 0%,100% {transform:translateY(0)} 50% {transform:translateY(-24px)} }
@keyframes idle-rock { 0%,50%,100% {transform:rotate(0)} 25% {transform:rotate(1.8deg)} 75% {transform:rotate(-1.8deg)} }
@keyframes idle-ring { 0%,50%,100% {transform:rotate(0)} 25% {transform:rotate(-2deg)} 75% {transform:rotate(2deg)} }
@keyframes hover-jump { 0%,100% {transform:translateY(0)} 50% {transform:translateY(-62px)} }
@keyframes hover-rock { 0%,50%,100% {transform:rotate(0)} 25% {transform:rotate(-2deg)} 75% {transform:rotate(2deg)} }
@keyframes hover-ring { 0%,50%,100% {transform:rotate(0)} 25% {transform:rotate(1.2deg)} 75% {transform:rotate(-1.2deg)} }
@keyframes talk-float { 0%,50%,100% {transform:translateY(0)} 25%,75% {transform:translateY(-10px)} }
@keyframes talk-rock { 0%,50%,100% {transform:rotate(0)} 25% {transform:rotate(1.2deg)} 75% {transform:rotate(-1.2deg)} }
@keyframes talk-mouth { 0%,100% {transform:scaleY(.25)} 50% {transform:scaleY(1)} }
@keyframes think-float { 0%,100% {transform:translateY(0)} 50% {transform:translateY(-10px)} }
@keyframes think-rock { 0%,50%,100% {transform:rotate(-4deg)} 25% {transform:rotate(-2.5deg)} 75% {transform:rotate(-5.5deg)} }
@keyframes think-ring { 0%,50%,100% {transform:rotate(0)} 25% {transform:rotate(2.5deg)} 75% {transform:rotate(-2.5deg)} }
@keyframes think-blink { 0%,59.5%,70.5%,100% {transform:scaleY(1)} 65% {transform:scaleY(.08)} }
@keyframes work-float { 0%,100% {transform:translateY(0)} 50% {transform:translateY(-14px)} }
@keyframes work-rock { 0%,50%,100% {transform:rotate(0)} 25% {transform:rotate(2.5deg)} 75% {transform:rotate(-2.5deg)} }
@keyframes work-ring { 0%,50%,100% {transform:rotate(0)} 25% {transform:rotate(4deg)} 75% {transform:rotate(-4deg)} }
@keyframes cloud-float {0%,100%{transform:translateY(0)}50%{transform:translateY(-8px)}}
@keyframes laptop-type {0%,50%,100%{transform:translateY(0) rotate(0)}25%{transform:translateY(-2px) rotate(-.8deg)}75%{transform:translateY(-2px) rotate(.8deg)}}
.state-idle #float {animation:idle-float 2.4s ease-in-out infinite}
.state-idle #body-rock {animation:idle-rock 2.4s ease-in-out infinite}
.state-idle #orbit-back-motion,.state-idle #orbit-front-motion {animation:idle-ring 2.4s ease-in-out infinite}
.state-hover #float {animation:hover-jump .6s ease-in-out 1 both}
.state-hover #body-rock {animation:hover-rock .6s ease-in-out 1 both}
.state-hover #orbit-back-motion,.state-hover #orbit-front-motion {animation:hover-ring .6s ease-in-out 1 both}
.state-talking #float {animation:talk-float 1.2s ease-in-out infinite}
.state-talking #body-rock {animation:talk-rock 1.2s ease-in-out infinite}
.state-talking #mouth-smile,.state-thinking #mouth-smile {opacity:0}
.state-talking #mouth-talk {opacity:1;animation:talk-mouth .2s ease-in-out infinite}
.state-thinking #float {animation:think-float 3s ease-in-out infinite}
.state-thinking #body-rock {animation:think-rock 3s ease-in-out infinite}
.state-thinking #orbit-back-motion,.state-thinking #orbit-front-motion {animation:think-ring 3s ease-in-out infinite}
.state-thinking #eyes-motion {animation:think-blink 3s ease-in-out infinite}
.state-thinking #gaze {transform:translate(-12px,-19px)}
.state-thinking #mouth-think {opacity:1}
.state-thinking #thought-cloud {opacity:1;animation:cloud-float 3s ease-in-out infinite}
.state-working #float {animation:work-float 1s ease-in-out infinite}
.state-working #body-rock {animation:work-rock 1s ease-in-out infinite}
.state-working #orbit-back-motion,.state-working #orbit-front-motion {animation:work-ring 1.5s ease-in-out infinite}
.state-working #gaze {transform:translate(18px,13px)}
.state-working #laptop {opacity:1;animation:laptop-type 1s ease-in-out infinite}
@media (prefers-reduced-motion: reduce) {
  * {animation:none!important;transform:none!important}
  #mouth-smile {opacity:1!important} #mouth-talk,#mouth-think {opacity:0!important}
  #thought-cloud,#laptop {opacity:0!important}
}
'''

def animated_svg(state):
    if state != 'busy':
        return SVG.replace('role="img"', f'class="state-{state}" role="img"', 1).replace('<defs>', '<style>' + CSS + '</style>\n  <defs>', 1)
    # Two complete poses alternate without requiring scripts or a browser-specific timer.
    inner = SVG[SVG.index('  <defs>'):SVG.rindex('</svg>')]
    def version(name):
        renamed = inner
        tree = ET.fromstring(SVG)
        ids = [e.attrib['id'] for e in tree.iter() if 'id' in e.attrib]
        for identifier in sorted(ids, key=len, reverse=True):
            renamed = renamed.replace('id="' + identifier + '"', 'id="' + name + '-' + identifier + '"')
            renamed = renamed.replace('url(#' + identifier + ')', 'url(#' + name + '-' + identifier + ')')
        return renamed
    busy_css = ''
    for name in ('thinking', 'working'):
        only = CSS
        for identifier in sorted([e.attrib['id'] for e in ET.fromstring(SVG).iter() if 'id' in e.attrib], key=len, reverse=True):
            only = only.replace('#' + identifier, '#' + name + '-' + identifier)
        busy_css += '\n' + only
    busy_css += '''
@keyframes busy-think {0%,49.999%{opacity:1}50%,100%{opacity:0}}
@keyframes busy-work {0%,49.999%{opacity:0}50%,100%{opacity:1}}
.busy-thinking {animation:busy-think 6s steps(1,end) infinite}
.busy-working {opacity:0;animation:busy-work 6s steps(1,end) infinite}
@media (prefers-reduced-motion: reduce) {
 .busy-thinking {animation:none!important;opacity:1!important}
 .busy-working {animation:none!important;opacity:0!important}
 [id$="mouth-smile"]{opacity:1!important}
 [id$="mouth-think"],[id$="mouth-talk"]{opacity:0!important}
 [id$="thought-cloud"],[id$="laptop"]{opacity:0!important}
}
'''
    return '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1254 1254" width="1254" height="1254" role="img" aria-label="Dibby alternates thinking and working every three seconds"><style>' + busy_css + '</style><g class="busy-thinking state-thinking">' + version('thinking') + '</g><g class="busy-working state-working">' + version('working') + '</g></svg>'

def scoped_svg(prefix):
    result = SVG
    ids = [e.attrib['id'] for e in ET.fromstring(SVG).iter() if 'id' in e.attrib]
    for identifier in sorted(ids, key=len, reverse=True):
        result = result.replace('id="' + identifier + '"', 'id="' + prefix + '-' + identifier + '" data-part="' + identifier + '"')
        result = result.replace('url(#' + identifier + ')', 'url(#' + prefix + '-' + identifier + ')')
    return result.replace('aria-labelledby="dibby-title dibby-desc"', f'aria-labelledby="{prefix}-dibby-title {prefix}-dibby-desc"')

def make_preview():
    previews = ''.join(f'<figure><div class="sample" data-state="{s}"'+(' tabindex="0" role="button" aria-label="Replay Dibby hover jump"' if s == 'hover' else '')+f'>{scoped_svg("preview-"+s)}</div><figcaption>{s.capitalize()}</figcaption></figure>' for s in STATES if s != 'neutral')
    page = '''<!doctype html><html lang="en"><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"><title>Dibby — mascot motion</title>
<style>
body{margin:0;color:var(--foreground,#292137);font-family:var(--font-sans,system-ui,sans-serif);line-height:1.5}h1{font-size:26px;margin:0 0 4px}p{margin:0;color:var(--muted-foreground,#6e687a)}
.stage{display:flex;align-items:center;gap:24px;flex-wrap:wrap;margin:16px 0}.hero{width:240px;height:240px;background:#350b35;border-radius:24px;flex:none}.hero svg{width:100%;height:100%;display:block}
.controls{flex:1;min-width:190px}.buttons{display:flex;gap:8px;flex-wrap:wrap;margin:12px 0}button{font:inherit;padding:8px 12px;border:1px solid var(--border,#ded9e5);border-radius:8px;background:var(--card,#f7f4ee);color:var(--foreground,#292137);cursor:pointer}button[aria-pressed=true]{background:var(--primary,#6639a6);color:var(--primary-foreground,#fff)}button:focus-visible,.hero:focus-visible{outline:3px solid #25baac;outline-offset:3px}label{display:block;margin:12px 0;cursor:pointer}
.gallery{display:grid;grid-template-columns:repeat(5,minmax(0,1fr));gap:12px}figure{margin:0;text-align:center}figcaption{font-size:14px;margin-top:4px}.sample{background:var(--card,#f7f4ee);border-radius:var(--radius,14px);padding:4px}.sample[role=button]{cursor:pointer}.sample:focus-visible{outline:3px solid #25baac;outline-offset:2px}.sample svg{width:100%;height:auto;display:block}.foot{font-size:12px;margin-top:16px}.status{font-weight:600;color:var(--foreground,#292137)}
@media(max-width:480px){.gallery{grid-template-columns:repeat(2,minmax(0,1fr))}.stage{gap:12px}.hero{width:180px;height:180px}}
</style>
<h1>Dibby</h1><p>The drawing buddy, ready for a little conversation.</p>
<div class="stage"><div class="hero" tabindex="0" aria-label="Hover or focus to make Dibby jump">__SVG__</div><div class="controls"><p class="status" id="status" aria-live="polite">Idle</p><div class="buttons"><button type="button" id="busy" aria-pressed="false">Busy cycle</button><button type="button" id="talk" aria-pressed="false">Talking</button><button type="button" id="reset">Reset</button></div><label><input id="reduce" type="checkbox"> Reduce motion</label><p>Idle floats. Hover for a little jump.<br>Busy: thought cloud 3s, laptop 3s.</p></div></div>
<div class="gallery">__GALLERY__</div><p class="foot">Animation preview only. No provider calls. The PNG app icon is preserved unchanged; this is its editable vector companion.</p>
<script>__RIG__
const hero=document.querySelector('.hero'),svg=hero.querySelector('svg'),busyButton=document.querySelector('#busy'),talkButton=document.querySelector('#talk'),reduce=document.querySelector('#reduce'),status=document.querySelector('#status');
const preferences=matchMedia('(prefers-reduced-motion: reduce)');
const flags={talking:false,hovered:false,busy:false};let busyStart=0,hoverStart=0,lastState='',stateStart=performance.now();
function setButton(button,value){button.setAttribute('aria-pressed',String(value))}
busyButton.onclick=()=>{flags.busy=!flags.busy;busyStart=performance.now();setButton(busyButton,flags.busy)};
talkButton.onclick=()=>{flags.talking=!flags.talking;setButton(talkButton,flags.talking)};
document.querySelector('#reset').onclick=()=>{flags.talking=false;flags.busy=false;busyStart=0;setButton(busyButton,false);setButton(talkButton,false)};
let pointerInside=false,focused=false;function hover(){const next=pointerInside||focused;if(next&&!flags.hovered)hoverStart=performance.now();flags.hovered=next}
hero.onpointerenter=()=>{pointerInside=true;hover()};hero.onpointerleave=()=>{pointerInside=false;hover()};hero.onfocus=()=>{focused=true;hover()};hero.onblur=()=>{focused=false;hover()};
const previews=[...document.querySelectorAll('.sample')];const began=performance.now();
const jumpPreview=previews.find(p=>p.dataset.state==='hover');function replay(){jumpPreview.dataset.jumpStart=performance.now()}jumpPreview.onpointerenter=replay;jumpPreview.onfocus=replay;jumpPreview.onclick=replay;jumpPreview.onkeydown=e=>{if(e.key==='Enter'||e.key===' '){e.preventDefault();replay()}};
function frame(now){const reduced=reduce.checked||preferences.matches;const elapsed=Math.max(0,now-busyStart),hoverElapsed=Math.max(0,now-hoverStart);const state=DibbyRig.resolve({...flags,reducedMotion:reduced,busyElapsedMs:elapsed,hoverElapsedMs:hoverElapsed});if(state!==lastState){stateStart=now;lastState=state}const label=reduced?'Neutral · reduced motion':state[0].toUpperCase()+state.slice(1);if(status.textContent!==label)status.textContent=label;
const local=(state==='thinking'||state==='working')?elapsed%3000:state==='hover'?hoverElapsed:now-stateStart;DibbyRig.apply(svg,DibbyRig.sample(state,local,reduced));for(const p of previews){const name=p.dataset.state;const time=name==='hover'?(p.dataset.jumpStart?now-Number(p.dataset.jumpStart):600):now-began;DibbyRig.apply(p.querySelector('svg'),DibbyRig.sample(name,time,reduced))}requestAnimationFrame(frame)}requestAnimationFrame(frame);
</script></html>'''
    return page.replace('__SVG__', scoped_svg('hero')).replace('__GALLERY__', previews).replace('__RIG__', RIG)

async def main():
    for state in [s for s in STATES if s != 'neutral'] + ['busy']:
        (ASSETS / f'dibby-{state}.svg').write_text(animated_svg(state), encoding='utf-8')
    contract = {'schema_version': 2, 'name': 'Dibby', 'source': 'dibby.svg',
      'atlas': {'file': 'dibby-atlas.png', 'frame_width': TILE, 'frame_height': TILE, 'columns': COLS,
        'rows': ROWS, 'width': TILE*COLS, 'height': TILE*ROWS, 'frame_count': 181, 'premultiplied_alpha': False},
      'states': {name: {'first_frame': first, 'frame_count': count, 'duration_ms': duration, 'loop': count > 1 and name != 'hover'}
                 for name, (first, count, duration) in STATES.items()},
      'controller': {'priority': ['reduced_motion', 'talking', 'hover', 'busy', 'idle'],
        'busy': {'sequence': [{'state': 'thinking', 'duration_ms': 3000}, {'state': 'working', 'duration_ms': 3000}],
          'loop': True, 'clock': 'time since busy began; overlays do not reset'},
        'hover': {'duration_ms': 600, 'trigger': 'pointer or keyboard focus enters', 'retrigger': 'after leaving then entering', 'on_complete': 'resume underlying busy or idle; do not repeat while hovered'},
        'reduced_motion': 'show neutral frame 0; expose actual status as text'},
      'rig': {'view_box': [0, 0, 1254, 1254], 'pivot': [630, 640], 'part_pivots': {'laptop': [1050, 990]},
        'layers': ['float', 'body-rock', 'orbit-back-motion', 'orbit-front-motion', 'gaze', 'eyes-motion', 'mouth-smile', 'mouth-talk', 'mouth-think', 'thought-cloud', 'laptop']},
      'source_sha256': hashlib.sha256(SVG.encode()).hexdigest()}
    (ASSETS / 'dibby-motion.json').write_text(json.dumps(contract, indent=2)+'\n', encoding='utf-8')
    (ROOT / 'docs').mkdir(exist_ok=True)
    (ROOT / 'docs' / 'dibby-preview.html').write_text(make_preview(), encoding='utf-8')
    atlas = Image.new('RGBA', (TILE*COLS, TILE*ROWS))
    async with async_playwright() as p:
        browser = await p.chromium.launch(channel='chrome', headless=True)
        page = await browser.new_page(viewport={'width': TILE, 'height': TILE}, device_scale_factor=1, reduced_motion='no-preference')
        await page.set_content('<style>html,body{margin:0;background:transparent}svg{width:160px;height:160px;display:block}</style>'+SVG)
        await page.add_script_tag(content=RIG)
        for name, (first, count, duration) in STATES.items():
            for frame in range(count):
                sample_time = duration * frame / (count-1 if name == 'hover' else count)
                await page.evaluate('([state,time]) => DibbyRig.apply(document.querySelector("svg"),DibbyRig.sample(state,time))', [name, sample_time])
                data = await page.screenshot(omit_background=True)
                tile = Image.open(io.BytesIO(data)).convert('RGBA')
                index = first+frame
                atlas.paste(tile, ((index % COLS)*TILE, (index // COLS)*TILE))
            print(f'Rendered {name}: {count} frames', flush=True)
        await browser.close()
    atlas.save(ASSETS / 'dibby-atlas.png', optimize=True)
    print(f'Wrote atlas {atlas.size} and SVG/JSON/preview assets', flush=True)

if __name__ == '__main__':
    asyncio.run(main())
