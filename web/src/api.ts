import type {
  FeatureCommand,
  FeatureExecuteResponse,
  TaskCreateResponse,
  WebClientInfo,
  WebStateResponse,
} from "./types";

interface NavigatorUAData {
  brands?: Array<{ brand: string; version: string }>;
  platform?: string;
  mobile?: boolean;
  getHighEntropyValues?: (hints: string[]) => Promise<{
    model?: string;
    platform?: string;
    platformVersion?: string;
    fullVersionList?: Array<{ brand: string; version: string }>;
  }>;
}

let webClientInfoPromise: Promise<WebClientInfo> | null = null;

function getStoredWebClientId() {
  const key = "cobweb_web_client_id";
  const stored = window.localStorage.getItem(key);
  if (stored) return stored;

  const clientId = typeof globalThis.crypto?.randomUUID === "function"
    ? globalThis.crypto.randomUUID()
    : `web-${Date.now().toString(36)}-${Math.random().toString(36).slice(2)}`;
  window.localStorage.setItem(key, clientId);
  return clientId;
}

function readUaData() {
  return (navigator as Navigator & { userAgentData?: NavigatorUAData }).userAgentData;
}

function pickBrowserName(info: NavigatorUAData | undefined, userAgent: string) {
  const brands = info?.brands ?? [];
  const brand = brands.find((item) => !/Not.?A.?Brand/i.test(item.brand));
  if (brand?.brand) return brand.brand;
  if (userAgent.includes("Edg/")) return "Edge";
  if (userAgent.includes("CriOS/") || userAgent.includes("Chrome/")) return "Chrome";
  if (userAgent.includes("FxiOS/") || userAgent.includes("Firefox/")) return "Firefox";
  if (userAgent.includes("Safari/")) return "Safari";
  return "浏览器";
}

function pickPlatformName(info: NavigatorUAData | undefined, userAgent: string) {
  if (info?.platform) return info.platform;
  if (userAgent.includes("iPhone")) return "iPhone";
  if (userAgent.includes("iPad")) return "iPad";
  if (userAgent.includes("Android")) return "Android";
  if (userAgent.includes("Windows")) return "Windows";
  if (userAgent.includes("Mac OS X")) return "macOS";
  return undefined;
}

export function getWebClientInfo() {
  webClientInfoPromise ??= (async (): Promise<WebClientInfo> => {
    const uaData = readUaData();
    const userAgent = navigator.userAgent;
    let deviceModel: string | undefined;
    let platform = pickPlatformName(uaData, userAgent);
    let browser = pickBrowserName(uaData, userAgent);

    try {
      const highEntropy = await uaData?.getHighEntropyValues?.([
        "model",
        "platform",
        "platformVersion",
        "fullVersionList",
      ]);
      deviceModel = highEntropy?.model || undefined;
      platform = highEntropy?.platform || platform;
      const fullBrand = highEntropy?.fullVersionList?.find((item) => !/Not.?A.?Brand/i.test(item.brand));
      browser = fullBrand?.brand || browser;
    } catch {
      // 部分浏览器不开放高精度设备信息，降级到 User-Agent 可读信息。
    }

    const deviceName = deviceModel || platform || (uaData?.mobile ? "移动设备" : "Web 设备");
    return {
      clientId: getStoredWebClientId(),
      deviceName,
      deviceModel,
      platform,
      browser,
      userAgent,
    };
  })();

  return webClientInfoPromise;
}

async function requestJson<T>(path: string, options: RequestInit = {}): Promise<T> {
  let response: Response;
  try {
    response = await fetch(path, {
      headers: { "Content-Type": "application/json" },
      ...options,
    });
  } catch {
    throw new Error("连接失败");
  }

  if (!response.ok) {
    throw new Error(`请求失败：${response.status}`);
  }

  try {
    return await response.json() as T;
  } catch {
    throw new Error("响应格式异常");
  }
}

export async function fetchState() {
  const payload = await requestJson<WebStateResponse>("/web/api/state");
  if (!payload.success) {
    throw new Error(payload.msg || "状态刷新失败");
  }
  return payload;
}

export async function executeCommand(command: FeatureCommand) {
  return requestJson<FeatureExecuteResponse>("/web/api/features/execute", {
    method: "POST",
    body: JSON.stringify({ client_info: await getWebClientInfo(), ...command }),
  });
}

export async function createScheduledTask(command: FeatureCommand, delayMs: number) {
  const executeAtMs = Date.now() + delayMs;
  return requestJson<TaskCreateResponse>("/web/api/tasks/create", {
    method: "POST",
    body: JSON.stringify({
      client_info: await getWebClientInfo(),
      execute_at_ms: executeAtMs,
      ...command,
    }),
  });
}

export async function cancelScheduledTask(taskId: string) {
  return requestJson<TaskCreateResponse>("/web/api/tasks/cancel", {
    method: "POST",
    body: JSON.stringify({ client_info: await getWebClientInfo(), task_id: taskId }),
  });
}
