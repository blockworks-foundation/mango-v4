import typescriptEslint from "@typescript-eslint/eslint-plugin";
import globals from "globals";
import tsParser from "@typescript-eslint/parser";
import path from "node:path";
import { fileURLToPath } from "node:url";
import js from "@eslint/js";
import { FlatCompat } from "@eslint/eslintrc";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const compat = new FlatCompat({
    baseDirectory: __dirname,
    recommendedConfig: js.configs.recommended,
    allConfig: js.configs.all
});

export default [{
    ignores: ["ts/client/src/mango_v4.ts", "ts/client/src/scripts"],
}, ...compat.extends("eslint:recommended", "plugin:@typescript-eslint/recommended", "prettier"), {
    plugins: {
        "@typescript-eslint": typescriptEslint,
    },

    languageOptions: {
        globals: {
            ...globals.browser,
            ...globals.node,
        },

        parser: tsParser,
        ecmaVersion: 12,
        sourceType: "module",
    },

    rules: {
        "linebreak-style": ["error", "unix"],
        semi: ["error", "always"],
        "@typescript-eslint/no-non-null-assertion": 0,
        "@typescript-eslint/ban-ts-comment": 0,
        "@typescript-eslint/no-explicit-any": 0,
        "@typescript-eslint/explicit-function-return-type": "warn",
        "@typescript-eslint/no-unused-expressions": "warn",
        "@typescript-eslint/no-unused-vars": [
            "error",
            {
              "args": "all",
              "argsIgnorePattern": "^_",
              "caughtErrors": "all",
              "caughtErrorsIgnorePattern": "^_",
              "destructuredArrayIgnorePattern": "^_",
              "varsIgnorePattern": "^_",
              "ignoreRestSiblings": true
            }
        ]
    },
}];