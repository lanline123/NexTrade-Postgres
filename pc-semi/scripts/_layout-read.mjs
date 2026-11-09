import fs from 'node:fs';
const j = JSON.parse(fs.readFileSync('checks/round-2/capture.json', 'utf8'));
console.log('--- design tokens ---');
console.log(JSON.stringify(j.design, null, 1).slice(0, 2500));
console.log('--- layouts 1440 ---');
console.log(JSON.stringify(j.layouts?.['1440'] ?? Object.values(j.layouts || {})[0], null, 1).slice(0, 3000));
