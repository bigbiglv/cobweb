export interface PeripheralDevice {
  id: string;
  classType: string;
  name: string;
  status: string;
  batteryPercentage?: number | null;
  batteryStatus?: string | null;
}
