import { registerPcModule, type ModuleDef } from '../../contract/pcModuleContract';

export const PC_LOGIN_DEFAULTS: Record<string, any> = {
  loginMode: 2,
  headerHeight: 70,
  logoWidth: 140,
  logoHeight: 40,
  headerShadow: true,
  showWelcome: true,
  backgroundPic: { picUrl: '', picThumb: '' },
  backgroundColor: '',
  warpHeight: 620,
};

const PC_LOGIN_MODULE_DEF: ModuleDef = {
  type: 'pcLogin',
  label: '登录页',
  group: 'pcLogin',
  hiddenInLibrary: false,
  defaults: () => ({ ...PC_LOGIN_DEFAULTS }),
  settings: [],
};

registerPcModule(PC_LOGIN_MODULE_DEF);
export default PC_LOGIN_MODULE_DEF;
