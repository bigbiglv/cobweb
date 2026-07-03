export interface PeripheralDevice {
  id: string;
  classType: string;
  name: string;
  status: string;
  containerId?: string | null;
  batteryPercentage?: number | null;
  batteryStatus?: string | null;
}
