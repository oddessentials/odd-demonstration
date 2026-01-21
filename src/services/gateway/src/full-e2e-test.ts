/**
 * Full E2E Validation Test File
 * Created: 2026-01-20T22:59:00
 *
 * Tests all agents:
 * - Semgrep (static analysis)
 * - Anthropic Claude (opencode, pr_agent)
 * - Ollama local_llm (with warm model)
 */

// Security issues for static analysis
export function insecureEval(userInput: string): unknown {
    return eval(userInput); // semgrep should catch this
}

// Logic issues for AI review
export function calculatePercentage(value: number, total: number): number {
    return (value / total) * 100; // no zero check
}

// API issues for AI review
export async function fetchUserData(userId: string) {
    const response = await fetch(`/api/users/${userId}`);
    return response.json(); // no error handling, no type safety
}

// Logging issues
export function processCredentials(creds: { apiKey: string; secret: string }) {
    console.log('Processing:', creds); // logging sensitive data
    return creds;
}
