/**
 * Test file for Anthropic E2E testing
 * Contains intentional issues for AI review to find
 */

export function calculateDiscount(price: number, discount: number): number {
    // Potential bug: no validation of negative values
    return price - (price * discount);
}

export function processUserData(user: any) {
    // Security issue: using any type
    console.log('Processing user:', user.password); // Logging sensitive data
    return user;
}

export async function fetchData(url: string) {
    // Missing error handling
    const response = await fetch(url);
    return response.json();
}

export function divideNumbers(a: number, b: number): number {
    // Potential divide by zero
    return a / b;
}
