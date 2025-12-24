/** @type {import('dependency-cruiser').IConfiguration} */
module.exports = {
    forbidden: [
        {
            name: 'no-circular',
            severity: 'error',
            comment: 'Circular dependencies can cause issues with bundling and testing',
            from: {},
            to: {
                circular: true,
            },
        },
        {
            name: 'no-orphans',
            severity: 'warn',
            comment: 'Orphaned files should be removed or connected to the dependency graph',
            from: {
                orphan: true,
                pathNot: [
                    '(^|/)\\.[^/]+\\.(js|cjs|mjs|ts|cts|mts|json)$', // Config files
                    '\\.d\\.ts$', // Type definition files
                    '(^|/)__tests__/', // Test files
                ],
            },
            to: {},
        },
        {
            name: 'no-dev-deps-in-source',
            severity: 'error',
            comment: 'Do not import dev dependencies in production code',
            from: {
                path: '^(index\\.ts)$',
                pathNot: '__tests__',
            },
            to: {
                dependencyTypes: ['npm-dev'],
            },
        },
    ],
    options: {
        doNotFollow: {
            path: 'node_modules',
        },
        tsPreCompilationDeps: true,
        tsConfig: {
            fileName: 'tsconfig.json',
        },
        enhancedResolveOptions: {
            exportsFields: ['exports'],
            conditionNames: ['import', 'require', 'node', 'default'],
        },
        reporterOptions: {
            dot: {
                theme: {
                    graph: { rankdir: 'LR' },
                },
            },
        },
    },
};
