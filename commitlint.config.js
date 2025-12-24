// commitlint.config.js
// Enforces Conventional Commits: https://www.conventionalcommits.org/
module.exports = {
    extends: ['@commitlint/config-conventional'],
    rules: {
        // Type must be one of these
        'type-enum': [
            2,
            'always',
            [
                'feat',     // New feature
                'fix',      // Bug fix
                'docs',     // Documentation only
                'style',    // Formatting, no code change
                'refactor', // Code change that neither fixes a bug nor adds a feature
                'perf',     // Performance improvement
                'test',     // Adding or updating tests
                'build',    // Build system or external dependencies
                'ci',       // CI configuration
                'chore',    // Other changes that don't modify src or test files
                'revert',   // Reverts a previous commit
            ],
        ],
        // Subject must not be empty
        'subject-empty': [2, 'never'],
        // Type must not be empty
        'type-empty': [2, 'never'],
        // Subject max length
        'subject-max-length': [2, 'always', 100],
    },
};
