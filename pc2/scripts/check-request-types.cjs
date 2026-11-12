const path = require("node:path");
const ts = require("typescript");

const root = path.resolve(__dirname, "..");
const file = path.join(root, "tests/types/request-contracts.ts");
const config = ts.readConfigFile(path.join(root, "tsconfig.json"), ts.sys.readFile);
const parsed = config.error ? undefined : ts.parseJsonConfigFileContent(config.config, ts.sys, root);
let diagnostics = config.error ? [config.error] : [...parsed.errors];
if (!diagnostics.length) {
    const ambient = parsed.fileNames.filter((name) => name.endsWith(".d.ts") && !name.includes("/node_modules/"));
    const program = ts.createProgram([file, ...ambient], {
        ...parsed.options, noEmit: true, composite: false, incremental: false, skipLibCheck: true
    });
    const source = program.getSourceFile(file);
    if (!source) throw new Error("Request type assertions were not loaded.");
    diagnostics.push(...program.getOptionsDiagnostics(), ...program.getSyntacticDiagnostics(source), ...program.getSemanticDiagnostics(source));
}
const errors = ts.sortAndDeduplicateDiagnostics(diagnostics);
if (errors.length) {
    process.stderr.write(ts.formatDiagnostics(errors, {
        getCurrentDirectory: () => root, getCanonicalFileName: (name) => name, getNewLine: () => "\n"
    }));
    process.exitCode = 1;
}
console.log(`Request type assertions: ${errors.length} diagnostics. Run type-check separately for application diagnostics.`);
