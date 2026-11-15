/* Shared deterministic pose sampler for SVG previews and native atlas generation. */
(function (root) {
  'use strict';
  const durations = { neutral: 0, idle: 2400, hover: 600, talking: 1200, thinking: 3000, working: 3000 };
  const TAU = Math.PI * 2;
  function sample(state, elapsedMs, reducedMotion = false) {
    if (!(state in durations)) throw new Error('Unknown Dibby state: ' + state);
    const pose = { floatY: 0, bodyRotation: 0, ringRotation: 0, gazeX: 0, gazeY: 0,
      eyeScaleY: 1, mouthScaleY: 1, smileOpacity: 1, talkOpacity: 0, thinkOpacity: 0,
      cloudOpacity: 0, cloudY: 0, laptopOpacity: 0, laptopY: 0, laptopRotation: 0 };
    if (reducedMotion || state === 'neutral') return pose;
    if (state === 'hover' && elapsedMs >= durations.hover) return pose;
    const t = state === 'hover' ? Math.max(0, Math.min(1, elapsedMs / durations.hover))
      : ((elapsedMs % durations[state]) + durations[state]) % durations[state] / durations[state];
    const wave = Math.sin(TAU * t);
    if (state === 'idle') {
      pose.floatY = -12 * (1 - Math.cos(TAU * t));
      pose.bodyRotation = 1.8 * wave;
      pose.ringRotation = -2 * wave;
    } else if (state === 'hover') {
      pose.floatY = -62 * Math.pow(Math.sin(Math.PI * t), 2);
      pose.bodyRotation = -2 * wave;
      pose.ringRotation = 1.2 * wave;
    } else if (state === 'talking') {
      pose.floatY = -5 * (1 - Math.cos(TAU * t * 2));
      pose.bodyRotation = 1.2 * wave;
      pose.smileOpacity = 0; pose.talkOpacity = 1;
      pose.mouthScaleY = 0.25 + 0.75 * Math.pow(Math.sin(TAU * t * 3), 2);
    } else if (state === 'thinking') {
      pose.floatY = -5 * (1 - Math.cos(TAU * t));
      pose.bodyRotation = -4 + 1.5 * wave;
      pose.ringRotation = 2.5 * wave;
      pose.gazeX = -12; pose.gazeY = -19;
      pose.smileOpacity = 0; pose.thinkOpacity = 1;
      pose.cloudOpacity = 1; pose.cloudY = -4 * (1 - Math.cos(TAU * t));
      const distance = Math.abs(t - 0.65);
      if (distance < 0.055) pose.eyeScaleY = 1 - 0.92 * (1 + Math.cos(Math.PI * distance / 0.055)) / 2;
    } else if (state === 'working') {
      pose.floatY = -7 * (1 - Math.cos(TAU * t * 3));
      pose.bodyRotation = 2.5 * Math.sin(TAU * t * 3);
      pose.ringRotation = 4 * Math.sin(TAU * t * 2);
      pose.gazeX = 18 + 3 * wave; pose.gazeY = 13;
      pose.laptopOpacity = 1;
      pose.laptopY = -2 * (1 - Math.cos(TAU * t * 3));
      pose.laptopRotation = -0.8 * Math.sin(TAU * t * 3);
    }
    return pose;
  }
  function resolve({ reducedMotion = false, talking = false, hovered = false, hoverElapsedMs = 0, busy = false, busyElapsedMs = 0 } = {}) {
    if (reducedMotion) return 'neutral';
    if (talking) return 'talking';
    if (hovered && hoverElapsedMs < durations.hover) return 'hover';
    if (busy) return Math.floor(Math.max(0, busyElapsedMs) / 3000) % 2 === 0 ? 'thinking' : 'working';
    return 'idle';
  }
  function apply(svg, pose) {
    const el = id => svg.querySelector('[data-part="' + id + '"]') || svg.querySelector('[id="' + id + '"]');
    const fmt = n => Number(n.toFixed(4));
    el('float').setAttribute('transform', `translate(0 ${fmt(pose.floatY)})`);
    el('body-rock').setAttribute('transform', `rotate(${fmt(pose.bodyRotation)} 630 640)`);
    for (const id of ['orbit-back-motion', 'orbit-front-motion'])
      el(id).setAttribute('transform', `rotate(${fmt(pose.ringRotation)} 630 640)`);
    el('gaze').setAttribute('transform', `translate(${fmt(pose.gazeX)} ${fmt(pose.gazeY)})`);
    el('eyes-motion').setAttribute('transform', `translate(600 535) scale(1 ${fmt(pose.eyeScaleY)}) translate(-600 -535)`);
    el('mouth-talk').setAttribute('transform', `translate(655 645) scale(1 ${fmt(pose.mouthScaleY)}) translate(-655 -645)`);
    for (const [id, key] of [['mouth-smile', 'smileOpacity'], ['mouth-talk', 'talkOpacity'], ['mouth-think', 'thinkOpacity']])
      el(id).setAttribute('opacity', pose[key]);
    el('thought-cloud').setAttribute('opacity', pose.cloudOpacity);
    el('thought-cloud').setAttribute('transform', `translate(0 ${fmt(pose.cloudY)})`);
    el('laptop').setAttribute('opacity', pose.laptopOpacity);
    el('laptop').setAttribute('transform', `translate(0 ${fmt(pose.laptopY)}) rotate(${fmt(pose.laptopRotation)} 1050 990)`);
  }
  root.DibbyRig = Object.freeze({ durations: Object.freeze(durations), sample, resolve, apply });
})(typeof globalThis !== 'undefined' ? globalThis : window);
