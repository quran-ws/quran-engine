// The doc-block linter: jsdoc/require-jsdoc on what an app calls, for web/ and the React
// Native package. docs/standards/CODE-STYLE.md states the scope; `scripts/check.sh docblocks` runs it.
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import jsdoc from 'eslint-plugin-jsdoc';
import tsParser from '@typescript-eslint/parser';

// The rule's own node list stays empty: each block names what it checks in `contexts`. The fixer
// is off, because it writes empty blocks, and an empty block is not a doc.
const none = { ArrowFunctionExpression: false, ClassDeclaration: false, ClassExpression: false, FunctionDeclaration: false, FunctionExpression: false, MethodDefinition: false };
const requireDocs = contexts => ['error', { require: none, contexts, enableFixer: false }];

// web/qvp.js is a script that assigns one global, so an ES export check sees nothing in it.
// Its public surface is the object it assigns to global.QVP. The names come from that line, so
// a new export is checked with no edit here. A renamed entry (`Page: QvpPage`) names the value,
// the declaration in the file.
const qvpJs = readFileSync(fileURLToPath(new URL('../../web/qvp.js', import.meta.url)), 'utf8');
const assigned = qvpJs.match(/global\.QVP = \{([^}]*)\}/);
if (!assigned) throw new Error('web/qvp.js: no `global.QVP = { … }` line to read the exports from');
const values = assigned[1].split(',').map(entry => entry.split(':').pop().trim()).filter(Boolean);
if (!values.every(v => /^[A-Za-z_$][\w$]*$/.test(v))) throw new Error('web/qvp.js: every global.QVP entry must be a name or `key: name`');
const exported = `/^(${values.join('|')})$/`;

export default [
  { ignores: ['web/*.test.mjs'] },
  {
    // Every exported class, every method of one that does not start with `_`, and every other
    // exported name. A constructor is left out: the class block says how to make one.
    files: ['web/qvp.js'],
    languageOptions: { sourceType: 'script' },
    plugins: { jsdoc },
    rules: {
      'jsdoc/require-jsdoc': requireDocs([
        `ClassDeclaration[id.name=${exported}]`,
        `ClassDeclaration[id.name=${exported}] > ClassBody > MethodDefinition[kind!="constructor"]:not([key.name=/^_/])`,
        `Program > ExpressionStatement FunctionDeclaration[id.name=${exported}]`,
        `VariableDeclaration:has(> VariableDeclarator[id.name=${exported}])`,
      ]),
    },
  },
  {
    // The ES exports of the lite modules, and the public methods of an exported class.
    files: ['web/lite*.mjs'],
    plugins: { jsdoc },
    rules: {
      'jsdoc/require-jsdoc': requireDocs([
        'ExportNamedDeclaration > FunctionDeclaration',
        'ExportNamedDeclaration > ClassDeclaration',
        'ExportNamedDeclaration > ClassDeclaration > ClassBody > MethodDefinition[kind!="constructor"][key.type!="PrivateIdentifier"]',
      ]),
    },
  },
  {
    // The exported declarations, the public members of an exported class, and the members of the
    // exported `Qvp` object.
    files: ['packages/react-native/qvp-react-native/src/**/*.{ts,tsx}'],
    languageOptions: { parser: tsParser, parserOptions: { ecmaFeatures: { jsx: true } } },
    plugins: { jsdoc },
    rules: {
      'jsdoc/require-jsdoc': requireDocs([
        'ExportNamedDeclaration > FunctionDeclaration',
        'ExportNamedDeclaration > ClassDeclaration',
        'ExportNamedDeclaration > ClassDeclaration > ClassBody > MethodDefinition[kind!="constructor"]:not([accessibility="private"])',
        'ExportNamedDeclaration:has(> VariableDeclaration)',
        'ExportNamedDeclaration > TSInterfaceDeclaration',
        'ExportNamedDeclaration > TSTypeAliasDeclaration',
        'ExportNamedDeclaration VariableDeclarator[id.name="Qvp"] > ObjectExpression > Property',
      ]),
    },
  },
];
