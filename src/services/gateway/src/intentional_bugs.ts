/**
 * Intentional security bugs for AI review E2E testing.
 * This file contains vulnerabilities that should be detected by AI code review.
 */

// SQL Injection via string concatenation
export function unsafeQuery(userId: string): string {
    return `SELECT * FROM users WHERE id = '${userId}'`;
}

// XSS vulnerability - innerHTML with user input
export function renderUserContent(content: string): void {
    document.getElementById("output")!.innerHTML = content;
}

// Prototype pollution vulnerability
export function mergeConfig(target: Record<string, unknown>, source: Record<string, unknown>): void {
    for (const key in source) {
        target[key] = source[key]; // No __proto__ check
    }
}

// Hardcoded secret
const API_SECRET = "ghp_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx";

// Insecure random for security purposes
export function generateToken(): string {
    return Math.random().toString(36).substring(2);
}

// eval with user input
export function executeCode(code: string): unknown {
    return eval(code);
}

// Regex DoS (ReDoS)
export function validateEmail(email: string): boolean {
    const regex = /^([a-zA-Z0-9_\.\-])+\@(([a-zA-Z0-9\-])+\.)+([a-zA-Z0-9]{2,4})+$/;
    return regex.test(email);
}

// Command injection
import { exec } from "child_process";

export function runCommand(userInput: string): void {
    exec(`ls ${userInput}`, (error, stdout) => {
        console.log(stdout);
    });
}
