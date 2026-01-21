/**
 * E2E Test File for Anthropic + Ollama Integration
 * Created: 2026-01-20T22:30:00
 *
 * This file tests:
 * - Anthropic Claude integration with JSON fence stripping
 * - Ollama local_llm with codellama:7b model
 */

// Intentional issues for AI review to catch:

export function unsafeEval(code: string): unknown {
    // Security: eval is dangerous
    return eval(code);
}

export async function fetchWithoutErrorHandling(url: string) {
    // Missing try/catch
    const response = await fetch(url);
    return response.json();
}

export function divideUnsafe(a: number, b: number): number {
    // Logic: no zero check
    return a / b;
}

export function logSensitiveData(user: { password: string; ssn: string }) {
    // Security: logging sensitive data
    console.log('User data:', user);
}
