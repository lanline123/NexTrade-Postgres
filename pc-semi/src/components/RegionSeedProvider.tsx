'use client';

/** 顶栏地区种子 Context：值由 RootLayout 服务端解析后注入，客户端只读。 */

import { createContext, useContext, type ReactNode } from 'react';
import type { RegionSeed } from '@/lib/region-seed';

const RegionSeedContext = createContext<RegionSeed | null>(null);

export default function RegionSeedProvider({ seed, children }: { seed: RegionSeed | null; children: ReactNode }) {
  return <RegionSeedContext.Provider value={seed}>{children}</RegionSeedContext.Provider>;
}

export function useRegionSeed(): RegionSeed | null {
  return useContext(RegionSeedContext);
}
