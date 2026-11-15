"""Check Dibby's vector/atlas contract and exercise exported animation behavior."""
from pathlib import Path
import asyncio
import hashlib
import json
import subprocess
from xml.etree import ElementTree as ET

from PIL import Image
from playwright.async_api import async_playwright

ROOT = Path(__file__).resolve().parents[1]
ASSETS = ROOT / 'assets' / 'mascot'

async def main():
    icon = ROOT / 'assets' / 'app-icon.png'
    assert hashlib.sha256(icon.read_bytes()).hexdigest() == '0f10fc0e471899454346759399719374ad3c3e4728fd438e602142212dd50afb'
    manifest = json.loads((ASSETS / 'dibby-motion.json').read_text(encoding='utf-8'))
    assert manifest['schema_version'] == 2
    assert manifest['states']['hover']['loop'] is False
    assert manifest['states']['idle']['loop'] is True
    assert hashlib.sha256((ASSETS / 'dibby.svg').read_bytes()).hexdigest() == manifest['source_sha256']
    for path in ASSETS.glob('*.svg'):
        tree = ET.fromstring(path.read_text(encoding='utf-8'))
        ids = [e.attrib['id'] for e in tree.iter() if 'id' in e.attrib]
        assert len(ids) == len(set(ids)), path
        assert not any(e.tag.rsplit('}', 1)[-1] in ('image', 'script', 'foreignObject') for e in tree.iter()), path
    atlas = Image.open(ASSETS / manifest['atlas']['file']).convert('RGBA')
    a = manifest['atlas']
    assert atlas.size == (a['width'], a['height'])
    assert len(manifest['states']) == 6
    used = []
    for state, definition in manifest['states'].items():
        checksums = set()
        for i in range(definition['first_frame'], definition['first_frame']+definition['frame_count']):
            assert i < a['frame_count']
            used.append(i)
            x, y = i % a['columns'] * a['frame_width'], i // a['columns'] * a['frame_height']
            tile = atlas.crop((x, y, x+a['frame_width'], y+a['frame_height']))
            bounds = tile.getchannel('A').getbbox()
            assert bounds and bounds[0] > 0 and bounds[1] > 0 and bounds[2] < a['frame_width'] and bounds[3] < a['frame_height'], (state, i, bounds)
            assert tile.getpixel((0, 0))[3] == 0
            checksums.add(hashlib.sha256(tile.tobytes()).digest())
        assert len(checksums) > 12 if state != 'neutral' else len(checksums) == 1
    assert sorted(used) == list(range(a['frame_count']))
    print('PASS: unchanged selected icon; vector-only SVGs; 181 indexed, transparent, unclipped atlas frames.')

    js = r'''
require('./tools/dibby-rig.js');const r=globalThis.DibbyRig;
for(const [time,state] of [[0,'thinking'],[2999,'thinking'],[3000,'working'],[5999,'working'],[6000,'thinking'],[9000,'working']])
 if(r.resolve({busy:true,busyElapsedMs:time})!==state)throw Error('busy boundary '+time);
if(r.resolve({busy:true,hovered:true})!=='hover')throw Error('hover priority');
if(r.resolve({busy:true,hovered:true,talking:true})!=='talking')throw Error('talk priority');
if(r.resolve({busy:true,hovered:true,talking:true,reducedMotion:true})!=='neutral')throw Error('reduced motion');
if(r.resolve({hovered:true,hoverElapsedMs:600})!=='idle')throw Error('hover repeats');
if(r.resolve({hovered:true,hoverElapsedMs:900,busy:true,busyElapsedMs:3100})!=='working')throw Error('hover does not resume busy');
if(r.sample('hover',300).floatY!==-62 || r.sample('hover',600).floatY!==0 || r.sample('hover',900).floatY!==0)throw Error('jump/landing');
if(r.sample('idle',1200).floatY!==-24)throw Error('idle no longer floats');
if(r.sample('thinking',100).cloudOpacity!==1 || r.sample('thinking',100).laptopOpacity!==0)throw Error('thinking cloud');
if(r.sample('working',100).laptopOpacity!==1 || r.sample('working',100).cloudOpacity!==0)throw Error('working laptop');
for(const state of Object.keys(r.durations))for(let t=0;t<10000;t+=37)
 for(const value of Object.values(r.sample(state,t)))if(!Number.isFinite(value))throw Error('nonfinite pose');
console.log('PASS: 3-second cycle, one-shot jump/landing, floating idle, cloud/laptop visibility, reduced motion, finite poses.');
'''
    subprocess.run(['node', '-e', js], cwd=ROOT, check=True)
    async with async_playwright() as p:
        browser = await p.chromium.launch(channel='chrome', headless=True)
        page = await browser.new_page(viewport={'width': 360, 'height': 360}, reduced_motion='no-preference')
        for state in ('idle', 'hover', 'talking', 'thinking', 'working'):
            await page.set_content('<style>body{margin:0}svg{width:320px;height:320px}</style>'+(ASSETS/f'dibby-{state}.svg').read_text(encoding='utf-8'))
            def timescript(t):
                return f'document.getAnimations().forEach(a=>{{a.pause();a.currentTime={t}}})'
            await page.evaluate(timescript(0))
            first = await page.locator('svg').screenshot()
            await page.evaluate(timescript(330))
            second = await page.locator('svg').screenshot()
            assert first != second, state
            for identifier, expected in [('thought-cloud', state == 'thinking'), ('laptop', state == 'working')]:
                opacity = await page.locator('#'+identifier).evaluate('(e)=>getComputedStyle(e).opacity')
                assert opacity == ('1' if expected else '0'), (state, identifier, opacity)
            if state == 'hover':
                await page.evaluate(timescript(700))
                assert await page.locator('#float').evaluate('(e)=>new DOMMatrix(getComputedStyle(e).transform).m42') == 0
        await page.set_content((ASSETS/'dibby-busy.svg').read_text(encoding='utf-8'))
        for time, expected in ((0, 'thinking'), (2999, 'thinking'), (3000, 'working'), (5999, 'working'), (6000, 'thinking')):
            await page.evaluate('(t)=>document.getAnimations().forEach(a=>{a.pause();a.currentTime=t})', time)
            opacities = await page.evaluate('Object.fromEntries(["thinking","working"].map(s=>[s,getComputedStyle(document.querySelector(".busy-"+s)).opacity]))')
            assert opacities[expected] == '1' and opacities['working' if expected == 'thinking' else 'thinking'] == '0', (time, opacities)
        await page.emulate_media(reduced_motion='reduce')
        assert await page.evaluate('getComputedStyle(document.querySelector(".busy-working")).opacity') == '0'
        assert await page.evaluate('getComputedStyle(document.querySelector("#thinking-mouth-smile")).opacity') == '1'
        assert await page.evaluate('getComputedStyle(document.querySelector("#thinking-body-rock")).transform') == 'none'
        assert await page.locator('#thinking-thought-cloud').evaluate('(e)=>getComputedStyle(e).opacity') == '0'
        await page.emulate_media(reduced_motion='no-preference')
        await page.goto((ROOT/'docs'/'dibby-preview.html').as_uri())
        await page.get_by_role('button', name='Busy cycle', exact=True).click()
        assert await page.get_by_role('button', name='Busy cycle', exact=True).get_attribute('aria-pressed') == 'true'
        await page.get_by_role('button', name='Talking', exact=True).click()
        await page.wait_for_function('document.querySelector("#status").textContent==="Talking"')
        await page.get_by_label('Reduce motion', exact=True).check()
        await page.wait_for_function('document.querySelector("#status").textContent.includes("reduced motion")')
        await page.get_by_role('button', name='Reset', exact=True).click()
        await page.get_by_label('Reduce motion', exact=True).uncheck()
        await page.wait_for_function('document.querySelector("#status").textContent==="Idle"')
        ids = await page.locator('[id]').evaluate_all('(items)=>items.map(e=>e.id)')
        assert len(ids) == len(set(ids))
        await page.set_viewport_size({'width': 390, 'height': 844})
        assert await page.evaluate('document.documentElement.scrollWidth<=innerWidth')
        await page.clock.install()
        await page.goto((ROOT/'docs'/'dibby-preview.html').as_uri())
        await page.locator('.hero').focus()
        await page.clock.run_for(100)
        assert await page.locator('#status').text_content() == 'Hover'
        await page.clock.run_for(650)
        assert await page.locator('#status').text_content() == 'Idle'
        await page.clock.run_for(1200)
        assert await page.locator('#status').text_content() == 'Idle'
        await page.locator('#busy').focus()
        await page.locator('.hero').focus()
        await page.clock.run_for(100)
        assert await page.locator('#status').text_content() == 'Hover'
        await browser.close()
    print('PASS: five animated SVGs, visible props, CSS busy timing, one-shot hover/re-entry, reduced motion, preview controls, mobile width.')

if __name__ == '__main__':
    asyncio.run(main())
