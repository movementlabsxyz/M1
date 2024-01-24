let request = new Map();

export function canRequest(ip) {
    let callTime = request.get(ip);
    if (!callTime) {
        request.set(ip, Date.now());
        return true;
    }
    if (Date.now() - callTime < 10 * 1000) {
        return false;
    }
    request.set(ip, Date.now());
    return true;
}
