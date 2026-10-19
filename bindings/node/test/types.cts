import monoprint = require('monoprint');
const renderer = new monoprint.Renderer({ theme: 'monoprint' });
const png: Buffer = renderer.pngSync('code');
const html: Promise<string> = monoprint.html('code');
void [png, html];
