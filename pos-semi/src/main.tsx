import React from 'react';
import ReactDOM from 'react-dom/client';
import { BrowserRouter } from 'react-router-dom';
import './legacy/00-index.css';
import './legacy/01-base.css';
import './legacy/10-PosCart-B5-bAp4a.css';
import './legacy/10-CategoryTabs-00cP0C_0.css';
import './legacy/10-Products-BcKJ7D15.css';
import './legacy/10-PosProduct-CB6MOn16.css';
import './legacy/10-PosStepper-C_gV4orL.css';
import './legacy/10-PosBadge-CvGri7oz.css';
import './legacy/10-Price-Ck3OBRsS.css';
import './legacy/10-Index-BLnWcMg3.css';
import './legacy/10-topBarUser-BQVf0Pr3.css';
import './legacy/10-pageTopMenu-C6XAvlcp.css';
import './legacy/10-boundDevice-DUvARTNY.css';
import './legacy/10-base-xMgyvGsX.css';
import './legacy/10-PosPopup-Dd5TeEf4.css';
import './legacy/10-DialogForm-Ddl0OvgM.css';
import './legacy/10-SettleAccountsPopup-lIYH4o3c.css';
import './styles.css';
import App from './App';

const basename = import.meta.env.BASE_URL.replace(/\/$/, '') || '/';

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <BrowserRouter basename={basename}>
      <App />
    </BrowserRouter>
  </React.StrictMode>,
);
