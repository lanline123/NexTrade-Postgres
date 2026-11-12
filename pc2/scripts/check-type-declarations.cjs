const fs = require("node:fs");
const path = require("node:path");
const ts = require("typescript");

const root = path.resolve(__dirname, "..");
const typeDirectory = ["src/types", "types"].map((directory) => path.join(root, directory)).find((directory) => fs.existsSync(directory));
const formatHost = {
    getCanonicalFileName: (file) => file,
    getCurrentDirectory: () => root,
    getNewLine: () => "\n"
};

function report(diagnostics) {
    const errors = ts.sortAndDeduplicateDiagnostics(diagnostics);
    if (errors.length) {
        process.stderr.write(ts.formatDiagnostics(errors, formatHost));
        process.exitCode = 1;
    }
    return errors.length;
}

function main() {
    if (!typeDirectory) {
        throw new Error("No project types directory found.");
    }
    const config = ts.readConfigFile(path.join(root, "tsconfig.json"), ts.sys.readFile);
    if (config.error) {
        report([config.error]);
        return;
    }
    const parsed = ts.parseJsonConfigFileContent(config.config, ts.sys, root);
    if (report(parsed.errors)) return;

    const targets = ts.sys.readDirectory(typeDirectory, [".ts", ".tsx"], ["**/node_modules/**"]);
    if (!targets.length) {
        throw new Error("No project type files found.");
    }
    const ambientFiles = parsed.fileNames.filter((file) => file.endsWith(".d.ts") && !file.split(path.sep).includes("node_modules"));
    const program = ts.createProgram([...new Set([...targets, ...ambientFiles])], {
        ...parsed.options,
        noEmit: true,
        composite: false,
        incremental: false,
        skipLibCheck: false,
        skipDefaultLibCheck: false
    });
    const diagnostics = [...program.getOptionsDiagnostics()];
    for (const file of targets) {
        const sourceFile = program.getSourceFile(file);
        if (!sourceFile) throw new Error(`Type file was not loaded: ${file}`);
        diagnostics.push(...program.getSyntacticDiagnostics(sourceFile), ...program.getSemanticDiagnostics(sourceFile));
    }
    const count = report(diagnostics);
    process.stdout.write(`Checked ${targets.length} project type files; ${count} diagnostics.\n`);
}

try {
    main();
} catch (error) {
    process.stderr.write(`${error instanceof Error ? error.message : String(error)}\n`);
    process.exitCode = 1;
}
