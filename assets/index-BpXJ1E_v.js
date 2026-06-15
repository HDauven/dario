(function(){let e=document.createElement(`link`).relList;if(e&&e.supports&&e.supports(`modulepreload`))return;for(let e of document.querySelectorAll(`link[rel="modulepreload"]`))n(e);new MutationObserver(e=>{for(let t of e)if(t.type===`childList`)for(let e of t.addedNodes)e.tagName===`LINK`&&e.rel===`modulepreload`&&n(e)}).observe(document,{childList:!0,subtree:!0});function t(e){let t={};return e.integrity&&(t.integrity=e.integrity),e.referrerPolicy&&(t.referrerPolicy=e.referrerPolicy),e.crossOrigin===`use-credentials`?t.credentials=`include`:e.crossOrigin===`anonymous`?t.credentials=`omit`:t.credentials=`same-origin`,t}function n(e){if(e.ep)return;e.ep=!0;let n=t(e);fetch(e.href,n)}})();var e={local:`dusk:0`,mainnet:`dusk:1`,testnet:`dusk:2`,devnet:`dusk:3`},t={USER_REJECTED:4001,UNAUTHORIZED:4100,UNSUPPORTED:4200,DISCONNECTED:4900,INTERNAL:-32603,INVALID_PARAMS:-32602,METHOD_NOT_FOUND:-32601},n=class extends Error{code;data;constructor(e,t){super(e),this.name=`DuskSdkError`,t?.cause!==void 0&&(this.cause=t.cause),t?.code!==void 0&&(this.code=t.code),t?.data!==void 0&&(this.data=t.data)}},r=class extends n{constructor(e=`Dusk Wallet not detected`){super(e,{code:t.UNSUPPORTED}),this.name=`DuskWalletNotInstalledError`}},i=class extends n{constructor(e=`Dusk Wallet does not support this method`){super(e,{code:t.UNSUPPORTED}),this.name=`DuskWalletUnsupportedMethodError`}},a=class extends n{constructor(e=`Dusk Wallet is locked or the site is not connected`){super(e,{code:t.UNAUTHORIZED}),this.name=`DuskWalletUnauthorizedError`}},o=class extends n{constructor(e=`User rejected the request`){super(e,{code:t.USER_REJECTED}),this.name=`DuskWalletUserRejectedError`}},s=class extends n{constructor(e=`Dusk Wallet provider is disconnected`){super(e,{code:t.DISCONNECTED}),this.name=`DuskWalletDisconnectedError`}},c=class extends n{constructor(e=`Select a Dusk wallet provider before making requests`){super(e,{code:t.UNSUPPORTED}),this.name=`DuskWalletProviderSelectionError`}},l=class extends n{constructor(e=`Requested Dusk wallet provider is not available`){super(e,{code:t.UNSUPPORTED}),this.name=`DuskWalletProviderNotFoundError`}};function u(e){return typeof e==`object`&&!!e&&`message`in e&&typeof e.message==`string`&&(`code`in e?typeof e.code==`number`:!0)}function d(e,t=`Unknown error`){if(u(e)||e instanceof Error)return e;if(typeof e==`string`)return new n(e);try{return new n(JSON.stringify(e))}catch{return new n(t)}}var f=`dusk:requestProvider`,p=`dusk:announceProvider`,m=`dusk.connect.selectedProvider`;function h(e){return typeof e==`string`?e.trim():``}function g(e){return{uuid:e.uuid,name:e.name,icon:e.icon,rdns:e.rdns}}function _(e){return e&&typeof e==`object`&&e.isDusk===!0&&typeof e.request==`function`&&typeof e.on==`function`}function v(e){return e&&typeof e==`object`&&h(e.uuid).length>0&&h(e.name).length>0&&typeof e.icon==`string`&&h(e.rdns).length>0}function y(e){return e&&typeof e==`object`&&v(e.info)&&_(e.provider)}function b(e){return{uuid:h(e.uuid),name:h(e.name),icon:String(e.icon??``),rdns:h(e.rdns).toLowerCase()}}function x(e){return{info:b(e.info),provider:e.provider}}function S(){return new Event(f)}function C(e={}){if(typeof window>`u`)return Promise.resolve([]);let t=Math.max(0,e.timeoutMs??40);return new Promise(n=>{let r=new Map,i=0,a=()=>{window.removeEventListener(p,o),i&&window.clearTimeout(i),e.signal?.removeEventListener(`abort`,a),n([...r.values()].map(e=>({info:g(e.info),provider:e.provider})).sort((e,t)=>e.info.name.localeCompare(t.info.name)))},o=e=>{let t=e.detail;if(!y(t))return;let n=x(t);n.info.uuid&&r.set(n.info.uuid,n)};if(e.signal?.aborted){a();return}if(window.addEventListener(p,o),e.signal?.addEventListener(`abort`,a,{once:!0}),window.dispatchEvent(S()),t===0){queueMicrotask(a);return}i=window.setTimeout(a,t)})}async function ee(e={}){let t=e.timeoutMs??2e3,n=e.intervalMs??50,r=await C({timeoutMs:0});if(r.length||typeof window>`u`||t<=0)return r;let i=Date.now();for(;;){let e=t-(Date.now()-i);if(e<=0)return[];let r=await C({timeoutMs:Math.min(n,e)});if(r.length)return r}}function w(e,t={}){if(typeof window>`u`)return()=>{};let n=t=>{let n=t.detail;y(n)&&e(x(n))};return window.addEventListener(p,n),t.requestOnStart!==!1&&window.dispatchEvent(S()),()=>{window.removeEventListener(p,n)}}function T(e){return typeof e==`string`&&/^[0-9a-fA-F]+$/.test(e)&&e.length%2==0}function E(e){let t=String(e||``).trim();if((t.startsWith(`0x`)||t.startsWith(`0X`))&&(t=t.slice(2)),t===``)return new Uint8Array;if(!T(t))throw Error(`Invalid hex`);let n=new Uint8Array(t.length/2);for(let e=0;e<n.length;e++)n[e]=parseInt(t.slice(e*2,e*2+2),16);return n}function D(e){let t=e instanceof Uint8Array?e:new Uint8Array(e),n=``;for(let e=0;e<t.length;e++){let r=t[e]??0;n+=r.toString(16).padStart(2,`0`)}return n}function te(e){if(e==null)return new Uint8Array;if(e instanceof Uint8Array)return e;if(e instanceof ArrayBuffer||Array.isArray(e))return new Uint8Array(e);if(typeof e==`string`){let t=e.trim();if(t.startsWith(`0x`)||T(t))return E(t)}throw Error(`Unsupported byte encoding (use hex string, Uint8Array, ArrayBuffer, or number[])`)}function ne(e){let t=typeof e==`string`?E(e):e instanceof Uint8Array?e:new Uint8Array(e);if(t.length!==32)throw TypeError(`contractId must be 32 bytes (0x + 64 hex chars)`);return`0x`+D(t).toLowerCase()}var re=[],ie=e=>({installed:e,providerId:null,providerInfo:null,availableProviders:re,authorized:!1,accounts:[],profiles:[],chainId:null,selectedAddress:null,selectedProfile:null,node:null,capabilities:null,lastUpdated:Date.now()});function O(e){return{uuid:e.uuid,name:e.name,icon:e.icon,rdns:e.rdns}}function ae(e){return{...e,providerInfo:e.providerInfo?O(e.providerInfo):null,availableProviders:e.availableProviders.map(O),accounts:[...e.accounts],profiles:e.profiles.map(e=>({...e})),selectedProfile:e.selectedProfile?{...e.selectedProfile}:null,node:e.node?{...e.node}:null}}function oe(e,t){if(e===t)return!0;if(e.length!==t.length)return!1;for(let n=0;n<e.length;n++)if(e[n]!==t[n])return!1;return!0}function se(e,t){return e===t?!0:!e||!t?!1:e.uuid===t.uuid&&e.name===t.name&&e.icon===t.icon&&e.rdns===t.rdns}function ce(e,t){if(e===t)return!0;if(e.length!==t.length)return!1;for(let n=0;n<e.length;n++)if(!se(e[n]??null,t[n]??null))return!1;return!0}function le(e){let n=d(e);switch(n.code){case t.UNSUPPORTED:return new i(n.message);case t.DISCONNECTED:return new s(n.message);case t.UNAUTHORIZED:return new a(n.message);case t.USER_REJECTED:return new o(n.message);default:return n}}var ue=class{_provider=null;_state=ie(!1);_subs=new Set;_providers=new Map;_bound=!1;_destroyed=!1;_readyPromise;_stopDiscovery=null;_explicitProvider=!1;_rememberLastUsed=!0;_providerStorageKey=m;_preferredProviderId=null;_readySettled=!1;_profilesFrom(e){return Array.isArray(e)?e.map((e,t)=>{if(!e||typeof e!=`object`)return null;let n=e,r=typeof n.account==`string`?n.account.trim():``;if(!r)return null;let i=typeof n.shieldedAddress==`string`&&n.shieldedAddress.trim()?n.shieldedAddress.trim():void 0,a=this._state.profiles.find(e=>e.account===r);return{profileId:typeof n.profileId==`string`&&n.profileId.trim()?n.profileId.trim():a?.profileId??`profile:${t}`,account:r,...i?{shieldedAddress:i}:{}}}).filter(Boolean):[]}_setProfiles(e,t={}){let n=this._profilesFrom(e),r=n.map(e=>e.account),i=n[0]??null,a=r[0]??null,o=JSON.stringify(this._state.profiles)===JSON.stringify(n),s=oe(this._state.accounts,r),c=JSON.stringify(this._state.selectedProfile??null)===JSON.stringify(i??null);o&&s&&c&&this._state.selectedAddress===a||this._patch({profiles:n,accounts:r,selectedProfile:i,selectedAddress:a},t)}_setDisconnected(){!this._state.authorized&&this._state.accounts.length===0&&this._state.profiles.length===0&&this._state.selectedAddress===null&&this._state.selectedProfile===null||this._patch({authorized:!1,accounts:[],profiles:[],selectedAddress:null,selectedProfile:null})}_hydrateFromProvider(e,t={}){this._patch({installed:this._providers.size>0||!!this._provider,chainId:e.chainId??this._state.chainId,authorized:!!e.isAuthorized},t),Array.isArray(e.profiles)&&this._setProfiles(e.profiles,{notify:!1})}_onConnect=e=>{let t=e?.chainId??this._provider?.chainId??null;this._state.authorized&&this._state.chainId===t||this._patch({authorized:!0,chainId:t})};_onDisconnect=e=>{this._setDisconnected()};_onProfilesChanged=e=>{this._setProfiles(e)};_onChainChanged=e=>{typeof e!=`string`||e===this._state.chainId||this._patch({chainId:e})};_onNodeChanged=e=>{e&&typeof e==`object`&&this._patch({node:e,chainId:e.chainId??this._state.chainId})};_events=[[`connect`,this._onConnect],[`disconnect`,this._onDisconnect],[`profilesChanged`,this._onProfilesChanged],[`chainChanged`,this._onChainChanged],[`duskNodeChanged`,this._onNodeChanged]];constructor(e={}){this._explicitProvider=!!e.provider,this._rememberLastUsed=e.rememberLastUsedProvider!==!1,this._providerStorageKey=e.providerStorageKey||`dusk.connect.selectedProvider`,this._preferredProviderId=e.preferredProviderId&&String(e.preferredProviderId).trim()||(this._rememberLastUsed?this._readStoredProviderId():null),this._state=ie(!1),this._stopDiscovery=w(e=>{this._registerDiscoveredProvider(e,{notify:!1}),this._readySettled&&!this._provider&&!this._explicitProvider&&this._autoSelectDiscoveredProvider({notify:!1}),this._notify()}),e.provider&&_(e.provider)&&(this._provider=e.provider,this._registerExplicitProvider(e.provider,e.providerInfo??null,{notify:!1,persist:!1})),this._readyPromise=(async()=>{if(!this._provider){let t=e.waitForProvider===!1?await C({timeoutMs:0}):await ee(e.providerWaitOptions);for(let e of t)this._registerDiscoveredProvider(e,{notify:!1});this._autoSelectDiscoveredProvider({notify:!1})}this._provider?(this._bindProviderEvents(),this._hydrateFromProvider(this._provider,{notify:!1}),e.autoRefresh!==!1&&await this.refresh().catch(()=>{})):this._syncAvailableProviders({notify:!1}),this._readySettled=!0,this._notify()})()}_availableProviderInfos(){return[...this._providers.values()].map(e=>O(e.info)).sort((e,t)=>e.name.localeCompare(t.name))}_syncAvailableProviders(e={}){let t=this._availableProviderInfos(),n=t.length>0||!!this._provider;(n!==this._state.installed||!ce(this._state.availableProviders,t))&&this._patch({installed:n,availableProviders:t},e)}_registerExplicitProvider(e,t,n={}){if(t?.uuid){this._providers.set(t.uuid,{info:O(t),provider:e}),this._applySelectedProvider({info:t,provider:e},n),this._syncAvailableProviders({notify:!1});return}this._provider=e,this._patch({installed:!0,providerId:null,providerInfo:null,availableProviders:this._availableProviderInfos(),authorized:!1,accounts:[],profiles:[],selectedAddress:null,selectedProfile:null,chainId:e.chainId??null,node:null,capabilities:null},{notify:!1}),this._hydrateFromProvider(e,{notify:!1}),this._syncAvailableProviders({notify:!1}),n.notify!==!1&&this._notify()}_registerDiscoveredProvider(e,t={}){let n=O(e.info),r=this._providers.get(n.uuid);return r&&r.provider===e.provider&&se(r.info,n)?!1:(this._providers.set(n.uuid,{info:n,provider:e.provider}),this._syncAvailableProviders({notify:!1}),this._state.providerId===n.uuid&&this._provider!==e.provider&&this._applySelectedProvider({info:n,provider:e.provider},{notify:!1,persist:!1}),t.notify!==!1&&this._notify(),!0)}_readStoredProviderId(){if(!this._rememberLastUsed||typeof localStorage>`u`)return null;try{let e=localStorage.getItem(this._providerStorageKey);return(typeof e==`string`?e.trim():``)||null}catch{return null}}_writeStoredProviderId(e){if(!(!this._rememberLastUsed||typeof localStorage>`u`))try{e?localStorage.setItem(this._providerStorageKey,e):localStorage.removeItem(this._providerStorageKey)}catch{}}_applySelectedProvider(e,t={}){let n=e?.provider??null,r=e?O(e.info):null,i=r?.uuid??null,a=this._provider===n,o=se(this._state.providerInfo,r);a||(this._unbindProviderEvents(),this._provider=n,this._provider&&this._bindProviderEvents()),this._patch({installed:this._providers.size>0||!!n,providerId:i,providerInfo:r,authorized:!1,accounts:[],profiles:[],selectedAddress:null,selectedProfile:null,chainId:n?.chainId??null,node:null,capabilities:null,availableProviders:this._availableProviderInfos()},{notify:!1}),n&&this._hydrateFromProvider(n,{notify:!1}),t.persist!==!1&&(this._preferredProviderId=i,this._writeStoredProviderId(i)),t.notify!==!1&&(!a||!o)&&this._notify()}_autoSelectDiscoveredProvider(e={}){if(!(this._explicitProvider||this._provider)){if(this._preferredProviderId){let t=this._providers.get(this._preferredProviderId);if(t){let n={persist:!1};e.notify!==void 0&&(n.notify=e.notify),this._applySelectedProvider(t,n);return}}if(this._providers.size===1){let t=[...this._providers.values()][0]??null;if(t){let n={persist:!1};e.notify!==void 0&&(n.notify=e.notify),this._applySelectedProvider(t,n)}}}}_getProvider(){return this._provider||this._autoSelectDiscoveredProvider({notify:!1}),this._provider}_requireProvider(){let e=this._getProvider();if(e)return e;throw this._state.availableProviders.length>0?new c:new r}async ready(){return await this._readyPromise,this}get provider(){return this._provider}get providerInfo(){return this._state.providerInfo?O(this._state.providerInfo):null}get providers(){return this._state.availableProviders.map(O)}get state(){return ae(this._state)}async discoverProviders(e={}){let t=await C(e);for(let e of t)this._registerDiscoveredProvider(e,{notify:!1});return!this._provider&&!this._explicitProvider&&this._autoSelectDiscoveredProvider({notify:!1}),this._notify(),this.providers}async selectProvider(e){let t=String(e||``).trim();if(!t)throw new l;let n=this._providers.get(t);if(n||=(await this.discoverProviders({timeoutMs:50}),this._providers.get(t)),!n)throw new l(`Unknown Dusk wallet provider: ${t}`);return this._applySelectedProvider(n,{notify:!1}),this._provider&&await this.refresh().catch(()=>{}),this._notify(),this.state}subscribe(e){this._subs.add(e);try{e(this.state)}catch{}return()=>{this._subs.delete(e)}}async request(e,t){let n=this._requireProvider();try{return await n.request({method:e,params:t})}catch(e){throw le(e)}}async refresh(){let e=this._getProvider();if(!e)return this._syncAvailableProviders({notify:!1}),this._patch({providerId:null,providerInfo:null,authorized:!1,accounts:[],profiles:[],selectedAddress:null,selectedProfile:null,chainId:null,node:null,capabilities:null},{notify:!1}),this._notify(),this.state;let[t,n,r]=await Promise.all([this.request(`dusk_getCapabilities`).catch(()=>null),this.request(`dusk_chainId`).catch(()=>e.chainId??null),this.request(`dusk_profiles`).catch(()=>[])]),i=typeof n==`string`?n:e.chainId??null;return this._patch({chainId:i,capabilities:t,authorized:!!e.isAuthorized},{notify:!1}),this._setProfiles(r,{notify:!1}),this._notify(),this.state}async connect(e){return await this.requestProfiles(e)}async requestProfiles(e){let t=e&&Object.keys(e).length>0?e:void 0,n=await this.request(`dusk_requestProfiles`,t),r=this._profilesFrom(n);return this._patch({authorized:!0,chainId:this._provider?.chainId??this._state.chainId},{notify:!1}),this._setProfiles(r,{notify:!1}),this._notify(),r}async disconnect(){let e=await this.request(`dusk_disconnect`);return this._setDisconnected(),!!e}async getProfiles(){let e=await this.request(`dusk_profiles`),t=this._profilesFrom(e);return this._setProfiles(t,{notify:!1}),this._notify(),t}async getAccounts(){return(await this.getProfiles()).map(e=>e.account)}async getChainId(){return await this.request(`dusk_chainId`)}async switchChain(e){return await this.request(`dusk_switchNetwork`,[e])}async getPublicBalance(){return await this.request(`dusk_getPublicBalance`)}async requestShieldedAddress(e={}){let t=await this.request(`dusk_requestShieldedAddress`,e),n=typeof t==`string`?t:t?.address,r=typeof n==`string`?n.trim():``;if(!r)throw Error(`Wallet did not return a shielded receive address`);let i=typeof t==`object`&&t&&typeof t.profileId==`string`?t.profileId.trim():``,a=typeof t==`object`&&t&&typeof t.account==`string`?t.account.trim():e.account??this._state.selectedProfile?.account??``,o=typeof t==`object`&&t&&typeof t.chainId==`string`&&t.chainId.trim()?t.chainId.trim():``;if(i||a){let e=!1,t=this._state.profiles.length?this._state.profiles.map((t,n)=>{let o=i?t.profileId===i:t.account===a;return o&&(e=!0),o?{...t,shieldedAddress:r}:{...t,profileId:t.profileId||`profile:${n}`}}):[{profileId:i||this._state.selectedProfile?.profileId||`account:0:${a}`,account:a,shieldedAddress:r}],n=this._state.profiles.length>0&&!e&&a?[...t,{profileId:i||`account:${t.length}:${a}`,account:a,shieldedAddress:r}]:t;this._patch({authorized:!0,chainId:o||this._state.chainId},{notify:!1}),this._setProfiles(n,{notify:!1}),this._notify()}return r}async getGasPrice(e){return await this.request(`dusk_estimateGas`,e??{})}async getCapabilities(){return await this.request(`dusk_getCapabilities`)}async signMessage(e){return await this.request(`dusk_signMessage`,{message:e})}async signAuth(e){return await this.request(`dusk_signAuth`,e)}async sendTransaction(e){return await this.request(`dusk_sendTransaction`,this._normalizeTransactionParams(e))}async sendTransfer(e){return await this.sendTransaction({kind:`transfer`,...e})}async sendContractCall(e){return await this.sendTransaction({kind:`contract_call`,...e})}_normalizeTransactionParams(e){if(e?.kind===`transfer`){let t=e,n=String(t.privacy??``).trim();if(!n)throw TypeError(`privacy is required ("public" or "shielded")`);if(n!==`public`&&n!==`shielded`)throw TypeError(`privacy must be "public" or "shielded"`);return{...t,privacy:n}}if(e?.kind!==`contract_call`)return e;let t=e,n=String(t.fnName??``).trim();if(!n)throw TypeError(`fnName is required`);let r=String(t.privacy??``).trim();if(!r)throw TypeError(`privacy is required ("public" or "shielded")`);if(r!==`public`&&r!==`shielded`)throw TypeError(`privacy must be "public" or "shielded"`);return{...t,privacy:r,contractId:ne(t.contractId),fnName:n,fnArgs:`0x`+D(te(t.fnArgs)).toLowerCase()}}async watchAsset(e,t={}){(t.autoConnect??!0)&&!this._state.authorized&&await this.connect();let n=String(e?.type??``).trim().toUpperCase(),r=e?.options??{},i=ne(r.contractId),a={type:n,options:{...r,contractId:i}};if(n===`DRC721`){let e=r.tokenId;a.options.tokenId=typeof e==`bigint`?e.toString():String(e??``).trim()}return await this.request(`dusk_watchAsset`,a)}on(e,t){let n=this._getProvider();return n?(n.on(e,t),()=>n.off(e,t)):()=>{}}destroy(){this._destroyed||(this._destroyed=!0,this._stopDiscovery?.(),this._stopDiscovery=null,this._unbindProviderEvents(),this._subs.clear())}_bindProviderEvents(){if(!(this._bound||!this._provider)){this._bound=!0;for(let[e,t]of this._events)this._provider.on(e,t)}}_unbindProviderEvents(){if(!(!this._bound||!this._provider)){this._bound=!1;for(let[e,t]of this._events)this._provider.off(e,t)}}_patch(e,t={}){this._state={...this._state,...e,lastUpdated:Date.now()},t.notify!==!1&&this._notify()}_notify(){if(this._destroyed)return;let e=this.state;for(let t of this._subs)try{t(e)}catch{}}};function de(e){return new ue(e)}function k(e){return String(e||``).trim().replace(/\/+$/,``)}function fe(e){let t=String(e||``).trim();if(!t)return``;let n=t.indexOf(`:`);if(n<=0)return``;let r=t.slice(0,n).toLowerCase(),i=t.slice(n+1);return r!==`dusk`||!/^\d+$/.test(i)?``:`${r}:${i}`}function pe(e){return String(e||``).replace(/^0x/i,``)}function A(e){let t={};for(let[n,r]of Object.entries(e))r!==void 0&&(t[n]=r);return t}function me(e){return pe(String(e||``).trim())}function he(e){let t=String(e||``);return/missing\s+feed/i.test(t)||/M\s*i\s*s\s*s\s*i\s*n\s*g\s+f\s*e\s*e\s*d/i.test(t)}var ge=`1.0.0-rc.0`,_e=3e4;function ve(e){let t=new URL(e);return t.protocol=t.protocol===`https:`?`wss:`:`ws:`,t.pathname=`/on`,t.search=``,t.hash=``,t.toString()}function ye(e){let t=new DataView(e);if(t.byteLength<4)throw Error(`Invalid RUES frame`);let n=t.getUint32(0,!0);if(4+n>t.byteLength)throw Error(`Invalid RUES headers length`);let r=new Uint8Array(e,4,n),i=[];try{i=JSON.parse(new TextDecoder().decode(r))}catch{i=[]}let a=new Headers(i),o=new Uint8Array(e,4+n),s=String(a.get(`content-type`)||``);if(/json/i.test(s))try{return{headers:a,payload:JSON.parse(new TextDecoder().decode(o))}}catch{}return{headers:a,payload:o}}async function be(e,t,n,r){let i=new Headers;i.set(`rusk-version`,ge),i.set(`rusk-session-id`,n);let a={method:`GET`,headers:i};r&&(a.signal=r);let o=await e(t,a);if(!o.ok){let e=await o.text().catch(()=>``);throw Error(e||`Unable to subscribe (${o.status} ${o.statusText})`)}try{await o.body?.cancel()}catch{}}function xe(e){let t=e.fetch??fetch,n=()=>k(typeof e.baseUrl==`function`?e.baseUrl():e.baseUrl);return{getBaseUrl:n,contractCall:async(e,r,i,a={})=>{let o=n();if(!o)throw Error(`DuskNodeClient: baseUrl is empty`);let s=`${o}/on/contracts:${me(e)}/${String(r)}`,c=new Headers;c.set(`Content-Type`,`application/octet-stream`),c.set(`Accept`,`application/octet-stream`);let l=te(i),u=l.buffer.slice(l.byteOffset,l.byteOffset+l.byteLength),d=async e=>{e?c.set(`Rusk-feeder`,`true`):c.delete(`Rusk-feeder`);let n={method:`POST`,headers:c,body:u};a.signal&&(n.signal=a.signal);let r=[429,502,503,504];for(let e=0;e<3;e++){try{let i=await t(s,n);if(i.ok||e===2||!r.includes(i.status))return i;try{i.body?.cancel?.()}catch{}}catch(t){if(a.signal?.aborted||e===2)throw t}await new Promise(t=>setTimeout(t,250*2**e))}return await t(s,n)};if(a.feeder===!0){let e=await d(!0);if(!e.ok)throw Error(await e.text()||`HTTP ${e.status}`);return new Uint8Array(await e.arrayBuffer())}if(a.feeder===!1){let e=await d(!1);if(!e.ok)throw Error(await e.text()||`HTTP ${e.status}`);return new Uint8Array(await e.arrayBuffer())}let f=await d(!1);if(!f.ok){let e=await f.text();if(he(e)){if(f=await d(!0),!f.ok)throw Error(await f.text()||`HTTP ${f.status}`)}else throw Error(e||`HTTP ${f.status}`)}return new Uint8Array(await f.arrayBuffer())},waitForTxExecuted:async(e,r={})=>{let i=n();if(!i)throw Error(`DuskNodeClient: baseUrl is empty`);let a=pe(String(e||``).trim()).toLowerCase();if(!a)return null;if(typeof WebSocket>`u`)throw Error(`waitForTxExecuted requires WebSocket support (browser environment)`);let o=Number(r.timeoutMs??6e4),s=r.signal,c=ve(i),l=`${i}/on/transactions:${a}/Executed`;return await new Promise((e,n)=>{let r=!1,i=null,u=!1,d,f,p=new WebSocket(c);p.binaryType=`arraybuffer`;let m=()=>{if(!r){r=!0,f&&clearTimeout(f),d&&clearInterval(d);try{p.close()}catch{}s&&s.removeEventListener(`abort`,g)}},h=e=>{m(),n(e instanceof Error?e:Error(String(e)))},g=()=>{h(s?.reason??Error(`Aborted`))};if(s){if(s.aborted){g();return}s.addEventListener(`abort`,g,{once:!0})}f=setTimeout(()=>{m(),e(null)},o),p.addEventListener(`error`,()=>h(Error(`RUES websocket error`))),p.addEventListener(`close`,()=>{r||h(Error(`RUES websocket closed`))}),p.addEventListener(`message`,async n=>{try{if(!i){i=typeof n.data==`string`?n.data:String(n.data),d=setInterval(()=>{try{p.readyState===WebSocket.OPEN&&p.send(``)}catch{}},_e),await be(t,l,i,s),u=!0;return}if(!u||!(n.data instanceof ArrayBuffer))return;let{headers:r,payload:o}=ye(n.data),c=String(r.get(`content-location`)||``).toLowerCase();if(!c||!c.includes(a))return;m(),e({headers:r,payload:o})}catch(e){h(e)}})})}}}var Se=64*1024,Ce=2*1024*1024,we=6;function Te(e){return e instanceof Uint8Array?e:new Uint8Array(e)}function Ee(e){let t=e.toLowerCase();return t.includes(`buffer`)&&t.includes(`small`)||t.includes(`insufficient`)||t.includes(`out_size`)}async function De(e){let t=Te(e),n=await WebAssembly.instantiate(t,{env:{}}),r=(n.instance??n).exports;if(!r?.memory||typeof r.alloc!=`function`||typeof r.dealloc!=`function`)throw Error(`Invalid data-driver WASM: missing required exports (memory/alloc/dealloc)`);let{memory:i}=r,a=new TextEncoder,o=new TextDecoder,s=(e,t)=>{try{r.dealloc(e,t)}catch{}},c=e=>{let t=r.alloc(e.length);return new Uint8Array(i.buffer,t,e.length).set(e),[t,e.length]},l=(e,t)=>{let[n,r]=c(e);try{return t(n,r)}finally{s(n,r)}},u=(e,t)=>l(a.encode(e),t),d=(e,t)=>{let n=new DataView(i.buffer,e,4).getUint32(0,!0);if(n>t-4)throw Error(`Invalid output size: ${n}`);return new Uint8Array(i.buffer,e+4,n).slice()},f=()=>{let e=4*1024;for(let t=0;t<6;t++){let t=e,n=r.alloc(t);try{return r.get_last_error(n,t),o.decode(d(n,t))}catch(n){if(String(n?.message||``).includes(`Invalid output size`)&&t<64*1024){e=t*2;continue}return``}finally{s(n,t)}}return``},p=e=>{let t=Se;for(let n=0;n<we;n++){let n=t,i=r.alloc(n);try{let r=e(i,n);if(r===0)return d(i,n);let a=f();if(a&&Ee(a)&&n<Ce){t=Math.min(n*2,Ce);continue}throw Error(`FFI call failed (${r}): ${a||`unknown error`}`)}finally{s(i,n)}}throw Error(`FFI call failed: output buffer too small (max retries reached)`)},m=e=>JSON.parse(o.decode(e)),h=(e,t,n)=>u(t,(t,r)=>l(n,(n,i)=>p((a,o)=>e(t,r,n,i,a,o)))),g=(e,t,n)=>u(t,(t,r)=>u(n,(n,i)=>p((a,o)=>e(t,r,n,i,a,o))));return{encodeInputFn:(e,t)=>g(r.encode_input_fn,String(e),String(t)),decodeInputFn:(e,t)=>m(h(r.decode_input_fn,String(e),t)),decodeOutputFn:(e,t)=>m(h(r.decode_output_fn,String(e),t)),decodeEvent:(e,t)=>m(u(String(e),(e,n)=>l(t,(t,i)=>p((a,o)=>r.decode_event(e,n,t,i,a,o))))),getSchema:()=>m(p((e,t)=>r.get_schema(e,t))),getVersion:()=>o.decode(p((e,t)=>r.get_version(e,t))),init:()=>r.init?.()}}async function Oe(e,t={}){let n=await(t.fetch??fetch)(e,t.init);if(!n.ok)throw Error(`Failed to fetch data-driver wasm (${n.status} ${n.statusText})`);let r=await n.arrayBuffer(),i=await De(new Uint8Array(r));try{i.init?.()}catch{}return i}async function ke(e,t,n={}){n.refresh!==!1&&await e.refresh().catch(()=>{});let r=typeof t?.chainId==`string`?t.chainId.trim():``,i=typeof t?.nodeUrl==`string`?t.nodeUrl.trim():``;if(!r&&!i)throw Error(`ensureChain: expected { chainId } or { nodeUrl }`);if(r){let t=fe(r);if(!t)throw Error(`ensureChain: chainId must be CAIP-2 (dusk:<id>)`);let n=fe(await e.getChainId().catch(()=>e.state.chainId)??``);return n&&n===t?!1:(await e.switchChain({chainId:t}),!0)}let a=k(i),o=e.state.node?.nodeUrl?String(e.state.node.nodeUrl):``,s=k(o);if(s){if(n.strictNodeUrl){if(o.trim()===i)return!1}else if(s===a)return!1}return await e.switchChain({nodeUrl:i}),!0}function Ae(e){try{if(!e||typeof e!=`object`)return!0;let t=e;return!(t.success===!1||t.err||t.error||t.result?.err||t.result?.error)}catch{return!0}}function je(e){try{if(!e||typeof e!=`object`)return``;let t=e,n=t.err??t.error??t.result?.err??t.result?.error;if(!n)return``;if(typeof n==`string`)return n;if(typeof n?.message==`string`)return n.message;try{return JSON.stringify(n)}catch{return String(n)}}catch{return``}}function Me(e,t){let n=String(e??``);if(!t)return{hash:n,status:`timeout`,ok:!1,error:`Timed out waiting for tx execution (${n.slice(0,12)}…)`};let r=Ae(t.payload),i=r?``:je(t.payload);return{hash:n,status:r?`executed`:`failed`,ok:r,...i?{error:i}:{},event:t}}function Ne(e){return JSON.stringify(e,(e,t)=>typeof t==`bigint`?t.toString():t)}function Pe(e){let t=typeof e==`string`?E(e):e instanceof Uint8Array?e:new Uint8Array(e);if(t.length!==32)throw TypeError(`contractId must be 32 bytes (0x + 64 hex chars)`);let n=D(t).toLowerCase();return{idHexNo0x:n,idHex0x:`0x`+n}}function Fe(e,t,n){let r={...t.name?{contractName:t.name}:{},methodSig:t.methodSigs?.[e]??e};return n&&typeof n==`object`?{...r,...n}:n==null?r:{...r,display:n}}function j(e){return new Proxy({},{get:(t,n)=>e(String(n))})}function Ie(e){let{idHex0x:t,idHexNo0x:n}=Pe(e.contractId),r=Promise.resolve(e.driver),i=A({name:e.name,methodSigs:e.methodSigs}),a=async(e,t)=>{let n=await r,i=t==null?`null`:Ne(t);return n.encodeInputFn(String(e),i)},o=async()=>(await r).getSchema?.(),s=async()=>(await r).getVersion?.(),c=j(t=>async(i,o)=>{if(!e.node)throw Error(`contract.call requires a node client`);let s=await a(t,i),c=await e.node.contractCall(n,t,s,o);return(await r).decodeOutputFn(String(t),c)}),l=j(n=>async(r,o)=>{let s=`0x`+D(await a(n,r)),c={...e.defaultTx??{},...o??{},contractId:t,fnName:n,fnArgs:s};c.display=Fe(n,i,o?.display??e.defaultTx?.display);let l=String(c.privacy??``).trim();if(l!==`public`&&l!==`shielded`)throw TypeError(`privacy is required ("public" or "shielded")`);return c.privacy=l,c});return{id:t,schema:o,version:s,encode:a,call:c,tx:l,write:j(t=>async(n,r)=>{let i=e.wallet;if(!i)throw Error(`contract.write requires a wallet`);let a=r?.autoConnect??e.autoConnect??!0,o=r?.chain??e.chain;a&&!i.state.authorized&&await i.connect(),o&&await ke(i,o);let{autoConnect:s,chain:c,...u}=r??{},d=await l[t](n,u),f=await i.sendContractCall(d),p=String(f?.hash??``),m={status:`submitted`,hash:p,nonce:String(f?.nonce??``)},h=new Set,g=()=>{for(let e of h)try{e(m)}catch{}},_=e=>{if(m.status===e.status)if(e.status===`submitted`){if(m.nonce===e.nonce)return}else return;m=e,g()},v=e=>{let t=e;h.add(t);try{t(m)}catch{}return()=>{h.delete(t)}},y=null,b=async t=>y||(y=(async()=>{if(!e.node)throw Error("tx.wait requires a node client (pass `node` when creating the contract facade)");m.status===`submitted`&&_({status:`executing`,hash:p});let n=null,r=null;try{n=await e.node.waitForTxExecuted(p,A({timeoutMs:t?.timeoutMs,signal:t?.signal}))}catch(e){if(t?.signal?.aborted)throw e;r=e,n=null}let i=Me(p,n);return r&&i.status===`timeout`&&(i.error=`Unable to track tx execution: ${r instanceof Error?r.message:String(r)}`),_({status:i.status,hash:p,receipt:i}),i})(),y);return Object.assign(f,{wait:b,waitExecuted:e=>b(e),onStatus:v})})}}var Le=`https://testnet.nodes.dusk.network`;function Re(e){let t=String(e||``).trim(),n=t.toLowerCase().startsWith(`0x`)?t.slice(2):t;return/^[0-9a-f]{64}$/.test(n)}function ze(e,t){if(!t||typeof t!=`object`)throw Error(`contracts.${e} must be an object`);if(!Re(String(t.contractId||``).trim()))throw Error(`contracts.${e}.contractId must be a 32-byte hex string (0x + 64 hex chars)`);if(!String(t.driverUrl||``).trim())throw Error(`contracts.${e}.driverUrl is required`);if(t.methodSigs!==void 0){let n=t.methodSigs;if(!n||typeof n!=`object`||Array.isArray(n))throw Error(`contracts.${e}.methodSigs must be a record of strings`);for(let[t,r]of Object.entries(n))if(typeof t!=`string`||typeof r!=`string`)throw Error(`contracts.${e}.methodSigs must map fnName -> signature string`)}}function Be(e){return A({to:e?.to,privacy:e?.privacy,amount:e?.amount,deposit:e?.deposit,gas:e?.gas,display:e?.display})}function Ve(e={}){let t=e.wallet instanceof ue?e.wallet:de(e.wallet),n=()=>{let n=t.state.node?.nodeUrl;return k(String(n||e.nodeUrl||Le))},r=xe({baseUrl:n}),i={...e.contracts??{}};for(let[e,t]of Object.entries(i))ze(e,t);let a=new Map,o=async t=>{let n=String(t||``).trim();if(!n)throw Error(`driverUrl is required`);if(e.disableDriverCache)return await Oe(n);let r=a.get(n);if(r)return await r;let i=Oe(n);a.set(n,i);try{return await i}catch(e){throw a.delete(n),e}},s=new Map,c=(e,t,n)=>[String(e.contractId||``),String(e.driverUrl||``),String(e.name||``),t?.chainId?String(t.chainId):``,t?.nodeUrl?k(String(t.nodeUrl)):``,n?`1`:`0`].join(`|`),l=(e,n,i)=>Ie(A({contractId:e.contractId,driver:o(e.driverUrl),node:r,wallet:t,autoConnect:i,chain:n,name:e.name,methodSigs:e.methodSigs,defaultTx:e.defaultTx})),u=t=>{if(typeof t==`string`){let n=t,r=i[n];if(!r)throw Error(`Unknown contract preset: ${String(t)}`);let a=e.chain,o=e.autoConnect??!0,u=c(r,a,o),d=s.get(n);if(d&&d.key===u)return d.value;let f=l(r,a,o);return s.set(n,{key:u,value:f}),f}let n=t;if(!n)throw Error(`contract config is required`);if(!Re(String(n.contractId||``).trim()))throw Error(`contract.contractId must be a 32-byte hex string (0x + 64 hex chars)`);if(!String(n.driverUrl||``).trim())throw Error(`contract.driverUrl is required`);return l(n,n.chain??e.chain,n.autoConnect??e.autoConnect??!0)};return{wallet:t,get state(){return t.state},subscribe:t.subscribe.bind(t),connect:t.connect.bind(t),disconnect:t.disconnect.bind(t),switchChain:t.switchChain.bind(t),ready:()=>t.ready(),nodeUrl:n,waitForTx:(e,t)=>r.waitForTxExecuted(e,t),waitForTxReceipt:async(e,t)=>{let n=null,i=null;try{n=await r.waitForTxExecuted(e,A({timeoutMs:t?.timeoutMs,signal:t?.signal}))}catch(e){if(t?.signal?.aborted)throw e;i=e,n=null}let a=Me(e,n);return i&&a.status===`timeout`&&(a.error=`Unable to track tx execution: ${i instanceof Error?i.message:String(i)}`),a},ensureChain:(e,n)=>ke(t,e,n),driver:o,contract:u,readContract:async({contract:e,functionName:t,args:n,options:r})=>await u(e).call[String(t)](n,r),prepareContractCall:async e=>{let t=u(e.contract),n=Be(e);return await t.tx[String(e.functionName)](e.args,n)},writeContract:async e=>{let t=u(e.contract),n=A({...Be(e),autoConnect:e.autoConnect,chain:e.chain});return await t.write[String(e.functionName)](e.args,n)},contracts:i}}function M(e){return!e||!e.installed?`missing`:e.authorized?e.profiles?.length?`connected`:`locked`:`disconnected`}function He(e){if(!e)return``;let t=e.node?.networkName;return t&&typeof t==`string`?t:e.chainId?e.chainId:``}function Ue(e,t=6,n=4){return e?e.length<=t+n+3?e:`${e.slice(0,t)}…${e.slice(-n)}`:``}var We=`
  :host, .dconnect-overlay {
    /*
      Namespaced theme tokens to avoid collisions with host dApps.
      You can override these on :root or on <dusk-connect-button>.
    */

    --dconnect-font-sans: "Sohne", ui-sans-serif, system-ui, -apple-system, Segoe UI, Roboto, Helvetica, Arial;
    --dconnect-font-mono: "Sohne Mono", ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace;

    /* Dusk brand foundation */
    --dconnect-radius-sm: 4px;
    --dconnect-radius: 8px;
    --dconnect-radius-lg: 12px;

    --dconnect-background: #101010;
    --dconnect-foreground: #F2F0EB;

    --dconnect-card: #151518;
    --dconnect-card-foreground: #F2F0EB;

    --dconnect-popover: #1B1B1E;
    --dconnect-popover-foreground: #F2F0EB;

    --dconnect-primary: #71B1FF;
    --dconnect-primary-hover: #8EC3FF;
    --dconnect-primary-foreground: #101010;

    --dconnect-secondary: #151518;
    --dconnect-secondary-foreground: #F2F0EB;

    --dconnect-muted: #27272A;
    --dconnect-muted-foreground: #A8A5AF;

    --dconnect-accent: rgba(113, 177, 255, 0.14);
    --dconnect-accent-foreground: #71B1FF;

    --dconnect-destructive: #E37A7A;
    --dconnect-destructive-foreground: #101010;

    --dconnect-border: rgba(242, 240, 235, 0.10);
    --dconnect-border-strong: rgba(242, 240, 235, 0.22);
    --dconnect-input: #0C0C0E;
    --dconnect-ring: #71B1FF;

    --dconnect-ok: #6FBF8E;
    --dconnect-warn: #E8B96A;

    --dconnect-shadow: 0 16px 40px rgba(0, 0, 0, 0.52);
    --dconnect-shadow-soft: 0 4px 16px rgba(0, 0, 0, 0.36);
    --dconnect-shadow-hover: 0 18px 44px rgba(0, 0, 0, 0.58);

    /* SDK-specific derived tokens */
    --dconnect-overlay-bg: rgba(16, 16, 16, 0.72);
    --dconnect-radius-pill: 999px;
    --dconnect-shadow-focus: 0 0 0 4px rgba(113, 177, 255, 0.24);

    --dconnect-button-bg: #151518;
    --dconnect-button-border: rgba(242, 240, 235, 0.15);
    --dconnect-button-hover: #1B1B1E;
    --dconnect-control-primary-bg: #E2DFE9;
    --dconnect-control-primary-fg: #101010;
    --dconnect-control-primary-hover-bg: #EDEAF3;

    /*
      The default Dusk treatment is solid, not a gradient.
    */
    --dconnect-primary-gradient: var(--dconnect-primary);
    --dconnect-primary-gradient-hover: var(--dconnect-primary-hover);

    --dconnect-avatar-bg: rgba(113, 177, 255, 0.12);
    --dconnect-avatar-fg: var(--dconnect-primary);
    --dconnect-avatar-border: rgba(113, 177, 255, 0.22);
    --dconnect-avatar-gradient: var(--dconnect-avatar-bg);
    --dconnect-provider-icon-bg: var(--dconnect-muted);
    --dconnect-provider-icon-fg: var(--dconnect-foreground);
    --dconnect-provider-dusk-icon-fg: var(--dconnect-primary);
    --dconnect-logo-mark: url("data:image/svg+xml,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20viewBox%3D%220%200%201000%201000%22%3E%3Cpath%20d%3D%22M514%2C0.2c-36.9-1-72.9%2C2-107.5%2C8.5C175%2C52.4%2C0%2C255.8%2C0%2C500s175.1%2C447.5%2C406.6%2C491.3c30.2%2C5.8%2C61.5%2C8.8%2C93.4%2C8.8c282.9%2C0%2C510.9-235%2C499.6-520.4C989.2%2C218.7%2C775%2C7.2%2C514%2C0.2z%20M522.6%2C899.4c-8.5%2C0.5-14-9.2-8.7-16C596.1%2C777.5%2C645.2%2C644.5%2C645.2%2C500s-49-277.6-131.4-383.4c-5.2-6.8%2C0.1-16.5%2C8.6-16C733%2C112.3%2C900%2C286.6%2C900%2C500S733.1%2C887.6%2C522.6%2C899.4z%22%2F%3E%3C%2Fsvg%3E");

    --dconnect-ease: cubic-bezier(0.2, 0, 0, 1);
    --dconnect-ease-out: cubic-bezier(0.16, 1, 0.3, 1);
    --dconnect-dur-fast: 120ms;
    --dconnect-dur-base: 200ms;
    --dconnect-dur-slow: 360ms;
  }

  :host([theme="light"]),
  .dconnect-overlay[data-theme="light"] {
    color-scheme: light;

    --dconnect-background: #F7F6F3;
    --dconnect-foreground: #101010;

    --dconnect-card: #FFFFFF;
    --dconnect-card-foreground: #101010;

    --dconnect-popover: #FFFFFF;
    --dconnect-popover-foreground: #101010;

    --dconnect-primary: #71B1FF;
    --dconnect-primary-hover: #8EC3FF;
    --dconnect-primary-foreground: #101010;

    --dconnect-secondary: #FFFFFF;
    --dconnect-secondary-foreground: #101010;

    --dconnect-muted: #EDEAF3;
    --dconnect-muted-foreground: #636167;

    --dconnect-accent: rgba(113, 177, 255, 0.14);
    --dconnect-accent-foreground: #2F86E8;

    --dconnect-destructive: #C95050;
    --dconnect-destructive-foreground: #FFFFFF;

    --dconnect-border: rgba(16, 16, 16, 0.12);
    --dconnect-border-strong: rgba(16, 16, 16, 0.24);
    --dconnect-input: #FFFFFF;
    --dconnect-ring: #2F86E8;

    --dconnect-shadow: 0 18px 48px rgba(16, 16, 16, 0.16);
    --dconnect-shadow-soft: 0 5px 18px rgba(16, 16, 16, 0.10);
    --dconnect-shadow-hover: 0 18px 42px rgba(16, 16, 16, 0.18);
    --dconnect-shadow-focus: 0 0 0 4px rgba(47, 134, 232, 0.20);

    --dconnect-overlay-bg: rgba(16, 16, 16, 0.36);
    --dconnect-button-bg: #FFFFFF;
    --dconnect-button-border: rgba(16, 16, 16, 0.14);
    --dconnect-button-hover: #F1F0F4;
    --dconnect-control-primary-bg: #101010;
    --dconnect-control-primary-fg: #E2DFE9;
    --dconnect-control-primary-hover-bg: #1E1E22;

    --dconnect-avatar-bg: rgba(47, 134, 232, 0.12);
    --dconnect-avatar-fg: #2F86E8;
    --dconnect-avatar-border: rgba(47, 134, 232, 0.22);
    --dconnect-provider-icon-bg: #EDEAF3;
    --dconnect-provider-icon-fg: #101010;
    --dconnect-provider-dusk-icon-fg: #101010;
  }

  @media (prefers-color-scheme: light) {
    :host(:not([theme="dark"])),
    .dconnect-overlay:not([data-theme="dark"]) {
      color-scheme: light;

      --dconnect-background: #F7F6F3;
      --dconnect-foreground: #101010;

      --dconnect-card: #FFFFFF;
      --dconnect-card-foreground: #101010;

      --dconnect-popover: #FFFFFF;
      --dconnect-popover-foreground: #101010;

      --dconnect-primary: #71B1FF;
      --dconnect-primary-hover: #8EC3FF;
      --dconnect-primary-foreground: #101010;

      --dconnect-secondary: #FFFFFF;
      --dconnect-secondary-foreground: #101010;

      --dconnect-muted: #EDEAF3;
      --dconnect-muted-foreground: #636167;

      --dconnect-accent: rgba(113, 177, 255, 0.14);
      --dconnect-accent-foreground: #2F86E8;

      --dconnect-destructive: #C95050;
      --dconnect-destructive-foreground: #FFFFFF;

      --dconnect-border: rgba(16, 16, 16, 0.12);
      --dconnect-border-strong: rgba(16, 16, 16, 0.24);
      --dconnect-input: #FFFFFF;
      --dconnect-ring: #2F86E8;

      --dconnect-shadow: 0 18px 48px rgba(16, 16, 16, 0.16);
      --dconnect-shadow-soft: 0 5px 18px rgba(16, 16, 16, 0.10);
      --dconnect-shadow-hover: 0 18px 42px rgba(16, 16, 16, 0.18);
      --dconnect-shadow-focus: 0 0 0 4px rgba(47, 134, 232, 0.20);

      --dconnect-overlay-bg: rgba(16, 16, 16, 0.36);
      --dconnect-button-bg: #FFFFFF;
      --dconnect-button-border: rgba(16, 16, 16, 0.14);
      --dconnect-button-hover: #F1F0F4;
      --dconnect-control-primary-bg: #101010;
      --dconnect-control-primary-fg: #E2DFE9;
      --dconnect-control-primary-hover-bg: #1E1E22;

      --dconnect-avatar-bg: rgba(47, 134, 232, 0.12);
      --dconnect-avatar-fg: #2F86E8;
      --dconnect-avatar-border: rgba(47, 134, 232, 0.22);
      --dconnect-provider-icon-bg: #EDEAF3;
      --dconnect-provider-icon-fg: #101010;
      --dconnect-provider-dusk-icon-fg: #101010;
    }
  }

  @keyframes dconnect-panel-in {
    from {
      opacity: 0;
      transform: translateY(8px) scale(0.985);
    }

    to {
      opacity: 1;
      transform: translateY(0) scale(1);
    }
  }

  @keyframes dconnect-fade-in {
    from { opacity: 0; }
    to { opacity: 1; }
  }

  @keyframes dconnect-panel-out {
    from {
      opacity: 1;
      transform: translateY(0) scale(1);
    }

    to {
      opacity: 0;
      transform: translateY(6px) scale(0.985);
    }
  }

  @keyframes dconnect-fade-out {
    from { opacity: 1; }
    to { opacity: 0; }
  }

  @keyframes dconnect-logo-pulse {
    0%, 100% { transform: scale(1); }
    45% { transform: scale(1.08); }
  }

  /* Lightweight box-sizing reset (scoped) */
  :host, :host * {
    box-sizing: border-box;
  }

  .dconnect-overlay, .dconnect-overlay * {
    box-sizing: border-box;
  }

  @media (prefers-reduced-motion: reduce) {
    :host *,
    :host *::before,
    :host *::after,
    .dconnect-overlay,
    .dconnect-overlay *,
    .dconnect-overlay *::before,
    .dconnect-overlay *::after {
      animation-duration: 1ms !important;
      animation-iteration-count: 1 !important;
      transition-duration: 1ms !important;
      scroll-behavior: auto !important;
    }
  }
`,Ge={missing:`Wallet not installed`,disconnected:`Not connected`,locked:`Locked`,connected:`Connected`},Ke={missing:`Install wallet`,disconnected:`Connect wallet`,locked:`Unlock wallet`,connected:`Disconnect`};async function qe(e){try{return await navigator.clipboard?.writeText?.(e),!0}catch{}try{let t=document.createElement(`textarea`);t.value=e,t.style.position=`fixed`,t.style.top=`-9999px`,t.style.left=`-9999px`,document.body.appendChild(t),t.focus(),t.select();let n=document.execCommand(`copy`);return document.body.removeChild(t),n}catch{return!1}}function Je(e){return e.providerInfo?.name||`Choose wallet`}function Ye(e){let t=(e||``).trim();return t?/^connect\b/i.test(t)?t:`Connect ${t}`:`Connect wallet`}function Xe(e){let t=String(e.name||``).trim().toLowerCase(),n=String(e.rdns||``).trim().toLowerCase();return t===`dusk wallet`||n===`network.dusk.wallet`||n.endsWith(`.dusk.wallet`)}function Ze(e){let t=String(e.name||`Wallet`).trim().charAt(0).toUpperCase();return/^[A-Z0-9]$/.test(t)?t:`W`}function Qe(e){return String(e.rdns||``).toLowerCase().includes(`harbor`)?`#6FBF8E`:`#71B1FF`}function $e(e){let t=String(e.icon||``).trim();return Xe(e)?`<span class="dconnect-provider-mark dconnect-provider-dusk" aria-hidden="true"></span>`:t?`<img class="dconnect-provider-icon" src="${N(t)}" alt="" />`:`<span class="dconnect-provider-mark dconnect-provider-initial" style="--dconnect-provider-accent: ${Qe(e)}" aria-hidden="true">${Ze(e)}</span>`}function et(e,t={}){if(typeof window>`u`)return{open:()=>{},close:()=>{},destroy:()=>{},isOpen:()=>!1};let n=t.closeOnConnect!==!1,r=null,i=null,a=!1,o=!1,s=null,c=null,l=null,u=null,d=null,f=null,p=null,m=null,h=null,g=null,_=null,v=`
    ${We}

    .dconnect-overlay {
      position: fixed;
      inset: 0;
      z-index: 2147483647;
      display: flex;
      align-items: center;
      justify-content: center;
      background: var(--dconnect-overlay-bg);
      animation: dconnect-fade-in var(--dconnect-dur-base) var(--dconnect-ease-out) both;
      backdrop-filter: blur(2px);
      -webkit-backdrop-filter: blur(2px);
    }

    .dconnect-overlay[data-state="closing"] {
      pointer-events: none;
      animation: dconnect-fade-out 180ms var(--dconnect-ease) both;
    }

    .dconnect-modal {
      width: min(460px, calc(100vw - 32px));
      border-radius: var(--dconnect-radius-lg);
      border: 1px solid var(--dconnect-border);
      box-shadow: var(--dconnect-shadow);
      overflow: hidden;
      color: var(--dconnect-foreground);
      font-family: var(--dconnect-font-sans);
      background: var(--dconnect-card);
      animation: dconnect-panel-in var(--dconnect-dur-slow) var(--dconnect-ease-out) both;
      transition:
        border-color var(--dconnect-dur-base) var(--dconnect-ease),
        box-shadow var(--dconnect-dur-base) var(--dconnect-ease);
    }

    .dconnect-overlay[data-state="closing"] .dconnect-modal {
      animation: dconnect-panel-out 180ms var(--dconnect-ease) both;
    }

    .dconnect-header {
      display: flex;
      align-items: center;
      justify-content: space-between;
      gap: 12px;
      padding: 18px 20px;
      border-bottom: 1px solid var(--dconnect-border);
      background: var(--dconnect-card);
    }

    .dconnect-brand {
      display: flex;
      align-items: center;
      gap: 10px;
      min-width: 0;
    }

    .dconnect-mark {
      width: 22px;
      height: 32px;
      display: grid;
      place-items: center;
      color: var(--dconnect-foreground);
      flex: 0 0 auto;
      transition:
        color var(--dconnect-dur-base) var(--dconnect-ease),
        transform var(--dconnect-dur-fast) var(--dconnect-ease);
    }

    .dconnect-mark::before {
      content: "";
      width: 16px;
      height: 16px;
      display: block;
      background: currentColor;
      -webkit-mask: var(--dconnect-logo-mark) center / contain no-repeat;
      mask: var(--dconnect-logo-mark) center / contain no-repeat;
      transform-origin: center;
    }

    .dconnect-brand:hover .dconnect-mark {
      color: var(--dconnect-foreground);
      transform: translateY(-1px);
    }

    .dconnect-brand:hover .dconnect-mark::before {
      animation: dconnect-logo-pulse 540ms var(--dconnect-ease-out);
    }

    .dconnect-txt {
      min-width: 0;
    }

    .dconnect-title {
      margin: 0;
      font-size: 14px;
      font-weight: 500;
      line-height: 1.2;
    }

    .dconnect-sub {
      margin: 5px 0 0;
      font-size: 12px;
      line-height: 1.35;
      color: var(--dconnect-muted-foreground);
    }

    .dconnect-icon-btn {
      appearance: none;
      cursor: pointer;
      border: 1px solid transparent;
      background: transparent;
      color: var(--dconnect-foreground);
      width: 34px;
      height: 34px;
      border-radius: var(--dconnect-radius-sm);
      display: inline-flex;
      align-items: center;
      justify-content: center;
      transition:
        background var(--dconnect-dur-base) var(--dconnect-ease),
        transform var(--dconnect-dur-fast) var(--dconnect-ease),
        border-color var(--dconnect-dur-base) var(--dconnect-ease),
        box-shadow var(--dconnect-dur-base) var(--dconnect-ease);
    }

    .dconnect-icon-btn svg {
      width: 16px;
      height: 16px;
      stroke: currentColor;
    }

    .dconnect-icon-btn:hover {
      border-color: var(--dconnect-border-strong);
      background: var(--dconnect-button-hover);
      box-shadow: none;
    }

    .dconnect-icon-btn:active {
      transform: translateY(1px);
    }

    .dconnect-icon-btn:focus-visible {
      outline: none;
      box-shadow: var(--dconnect-shadow-focus);
    }

    .dconnect-body {
      padding: 16px 20px 20px;
      display: grid;
      gap: 0;
    }

    .dconnect-row {
      display: flex;
      justify-content: space-between;
      align-items: center;
      gap: 12px;
      min-height: 42px;
      padding: 10px 0;
      border-radius: 0;
      border: 0;
      border-bottom: 1px solid var(--dconnect-border);
      background: transparent;
    }

    .dconnect-row-data {
      transition:
        transform var(--dconnect-dur-fast) var(--dconnect-ease),
        background var(--dconnect-dur-base) var(--dconnect-ease),
        border-color var(--dconnect-dur-base) var(--dconnect-ease),
        box-shadow var(--dconnect-dur-base) var(--dconnect-ease);
    }

    .dconnect-row-data:hover {
      border-color: var(--dconnect-border);
      background: transparent;
      box-shadow: none;
      transform: none;
    }

    .dconnect-lab {
      font-family: var(--dconnect-font-mono);
      font-size: 10px;
      font-weight: 400;
      letter-spacing: 0.18em;
      text-transform: uppercase;
      color: var(--dconnect-muted-foreground);
    }

    .dconnect-val {
      font-size: 12px;
      font-family: var(--dconnect-font-mono);
      display: inline-flex;
      align-items: center;
      gap: 8px;
      min-width: 0;
      color: var(--dconnect-foreground);
    }

    .dconnect-val span {
      min-width: 0;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
      max-width: 260px;
    }

    .dconnect-copy {
      appearance: none;
      height: 28px;
      min-width: 28px;
      padding: 0 10px;
      border-radius: var(--dconnect-radius-sm);
      background: transparent;
      border: 1px solid var(--dconnect-border);
      color: var(--dconnect-foreground);
      font-size: 12px;
      font-weight: 500;
      cursor: pointer;
      transition:
        background var(--dconnect-dur-base) var(--dconnect-ease),
        transform var(--dconnect-dur-fast) var(--dconnect-ease),
        border-color var(--dconnect-dur-base) var(--dconnect-ease),
        box-shadow var(--dconnect-dur-base) var(--dconnect-ease);
    }

    .dconnect-copy:hover {
      background: var(--dconnect-button-hover);
      border-color: var(--dconnect-border-strong);
      box-shadow: var(--dconnect-shadow-soft);
    }

    .dconnect-copy:active {
      transform: translateY(1px);
    }

    .dconnect-copy:focus-visible {
      outline: none;
      box-shadow: var(--dconnect-shadow-focus);
    }

    .dconnect-section {
      display: grid;
      gap: 8px;
      margin-top: 14px;
    }

    .dconnect-section-label {
      font-family: var(--dconnect-font-mono);
      font-size: 10px;
      font-weight: 400;
      letter-spacing: 0.18em;
      text-transform: uppercase;
      color: var(--dconnect-muted-foreground);
    }

    .dconnect-section-label::before {
      content: "[ ";
    }

    .dconnect-section-label::after {
      content: " ]";
    }

    .dconnect-provider-list {
      display: grid;
      gap: 10px;
    }

    .dconnect-provider {
      appearance: none;
      width: 100%;
      display: flex;
      align-items: center;
      justify-content: space-between;
      gap: 12px;
      padding: 11px 12px;
      border-radius: var(--dconnect-radius-sm);
      border: 1px solid var(--dconnect-border);
      background: var(--dconnect-card);
      color: inherit;
      cursor: pointer;
      transition:
        transform var(--dconnect-dur-fast) var(--dconnect-ease),
        border-color var(--dconnect-dur-base) var(--dconnect-ease),
        box-shadow var(--dconnect-dur-base) var(--dconnect-ease),
        background var(--dconnect-dur-base) var(--dconnect-ease);
    }

    .dconnect-provider:hover {
      transform: translateY(-1px);
      border-color: var(--dconnect-border-strong);
      box-shadow: var(--dconnect-shadow-soft);
    }

    .dconnect-provider:focus {
      outline: none;
    }

    .dconnect-provider:focus-visible {
      outline: none;
      border-color: var(--dconnect-primary);
      box-shadow: var(--dconnect-shadow-focus);
    }

    .dconnect-provider[data-selected="true"] {
      border-color: var(--dconnect-border-strong);
      background: var(--dconnect-popover);
      box-shadow: inset 3px 0 0 var(--dconnect-primary);
    }

    .dconnect-provider-main {
      display: flex;
      align-items: center;
      gap: 10px;
      min-width: 0;
    }

    .dconnect-provider-icon,
    .dconnect-provider-mark {
      width: 40px;
      height: 40px;
      border-radius: var(--dconnect-radius-sm);
      flex: 0 0 auto;
      display: grid;
      place-items: center;
      object-fit: cover;
      background: var(--dconnect-provider-icon-bg);
      border: 1px solid var(--dconnect-border);
    }

    .dconnect-provider-initial {
      color: var(--dconnect-provider-icon-fg);
      font-family: var(--dconnect-font-mono);
      font-size: 14px;
      font-weight: 500;
      box-shadow: inset 4px 0 0 var(--dconnect-provider-accent);
    }

    .dconnect-provider-dusk {
      color: var(--dconnect-provider-dusk-icon-fg);
    }

    .dconnect-provider-dusk::before {
      content: "";
      width: 20px;
      height: 20px;
      display: block;
      background: currentColor;
      -webkit-mask: var(--dconnect-logo-mark) center / contain no-repeat;
      mask: var(--dconnect-logo-mark) center / contain no-repeat;
      transform-origin: center;
    }

    .dconnect-provider:hover .dconnect-provider-dusk::before {
      animation: dconnect-logo-pulse 540ms var(--dconnect-ease-out);
    }

    .dconnect-provider-copy {
      min-width: 0;
      display: grid;
      gap: 2px;
      text-align: left;
    }

    .dconnect-provider-name {
      font-size: 13px;
      font-weight: 500;
      min-width: 0;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }

    .dconnect-provider-rdns {
      font-family: var(--dconnect-font-mono);
      font-size: 11px;
      color: var(--dconnect-muted-foreground);
      min-width: 0;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }

    .dconnect-provider-tag {
      font-family: var(--dconnect-font-mono);
      font-size: 10px;
      font-weight: 400;
      letter-spacing: 0.12em;
      text-transform: uppercase;
      color: var(--dconnect-muted-foreground);
    }

    .dconnect-actions {
      display: flex;
      gap: 8px;
      margin-top: 12px;
    }

    .dconnect-btn {
      flex: 1;
      appearance: none;
      border: 1px solid var(--dconnect-border);
      border-radius: var(--dconnect-radius-sm);
      padding: 10px 12px;
      font-size: 13px;
      font-weight: 500;
      line-height: 1;
      cursor: pointer;
      user-select: none;
      -webkit-tap-highlight-color: transparent;
      background: var(--dconnect-secondary);
      color: var(--dconnect-secondary-foreground);
      transition:
        transform var(--dconnect-dur-fast) var(--dconnect-ease),
        background var(--dconnect-dur-base) var(--dconnect-ease),
        border-color var(--dconnect-dur-base) var(--dconnect-ease),
        box-shadow var(--dconnect-dur-base) var(--dconnect-ease);
    }

    .dconnect-btn:hover {
      transform: translateY(-1px);
      background: var(--dconnect-button-hover);
      border-color: var(--dconnect-border-strong);
      box-shadow: var(--dconnect-shadow-hover);
    }

    .dconnect-btn:active {
      transform: translateY(1px);
    }

    .dconnect-btn:focus-visible {
      outline: none;
      box-shadow: var(--dconnect-shadow-focus);
    }

    .dconnect-btn:disabled {
      opacity: 0.55;
      cursor: not-allowed;
      box-shadow: none;
    }

    .dconnect-btn-primary {
      border-color: transparent;
      color: var(--dconnect-control-primary-fg);
      background: var(--dconnect-control-primary-bg);
    }

    .dconnect-btn-primary:hover {
      background: var(--dconnect-control-primary-hover-bg);
      box-shadow: var(--dconnect-shadow);
    }

    .dconnect-btn-destructive {
      border-color: transparent;
      background: var(--dconnect-destructive);
      color: var(--dconnect-destructive-foreground);
    }

    .dconnect-hint {
      margin-top: 2px;
      font-size: 11.5px;
      line-height: 1.35;
      color: var(--dconnect-muted-foreground);
      min-height: 16px;
    }
  `,y=e=>{if(!h)return;let t=e.availableProviders??[];if(t.length===0){h.innerHTML=``,h.hidden=!0;return}h.hidden=!1,h.innerHTML=t.map(t=>{let n=t.uuid===e.providerId;return`
          <button
            class="dconnect-provider"
            type="button"
            data-action="select-provider"
            data-provider-id="${N(t.uuid)}"
            data-selected="${n?`true`:`false`}"
          >
            <span class="dconnect-provider-main">
              ${$e(t)}
              <span class="dconnect-provider-copy">
                <span class="dconnect-provider-name">${N(t.name)}</span>
                <span class="dconnect-provider-rdns">${N(t.rdns)}</span>
              </span>
            </span>
            <span class="dconnect-provider-tag">${n?`Selected`:`Available`}</span>
          </button>
        `}).join(``)},b=()=>{r||(r=document.createElement(`div`),r.className=`dconnect-overlay`,(t.theme===`dark`||t.theme===`light`)&&(r.dataset.theme=t.theme),r.tabIndex=-1,r.innerHTML=`
      <style>${v}</style>
      <div class="dconnect-modal" role="dialog" aria-modal="true">
        <div class="dconnect-header">
          <div class="dconnect-brand">
            <div class="dconnect-mark" aria-hidden="true"></div>
            <div class="dconnect-txt">
              <div class="dconnect-title" id="dconnectTitle">${N(Ye(t.appName))}</div>
              <div class="dconnect-sub">Choose a Dusk wallet, then approve access for this site.</div>
            </div>
          </div>
          <button class="dconnect-icon-btn" type="button" data-action="close" aria-label="Close">
            <svg aria-hidden="true" viewBox="0 0 24 24" fill="none" stroke-width="1.5" stroke-linecap="square">
              <path d="M6 6l12 12M18 6L6 18"></path>
            </svg>
          </button>
        </div>
        <div class="dconnect-body">
          <div class="dconnect-row dconnect-row-data"><div class="dconnect-lab">Status</div><div class="dconnect-val" id="dwcStatus">—</div></div>
          <div class="dconnect-row dconnect-row-data"><div class="dconnect-lab">Wallet</div><div class="dconnect-val"><span id="dwcWallet">—</span></div></div>
          <div class="dconnect-row dconnect-row-data">
            <div class="dconnect-lab">Profile</div>
            <div class="dconnect-val"><span id="dwcAccount">—</span><button class="dconnect-copy" id="dwcCopy" type="button" data-action="copy" hidden>Copy</button></div>
          </div>
          <div class="dconnect-row dconnect-row-data"><div class="dconnect-lab">Network</div><div class="dconnect-val" id="dwcNetwork">—</div></div>
          <div class="dconnect-section">
            <div class="dconnect-section-label">Wallets</div>
            <div class="dconnect-provider-list" id="dwcProviders" hidden></div>
          </div>
          <div class="dconnect-actions">
            <button class="dconnect-btn dconnect-btn-primary" id="dwcPrimary" type="button" data-action="primary">—</button>
          </div>
          <div class="dconnect-hint" id="dwcHint"></div>
        </div>
      </div>
    `,l=r.querySelector(`#dconnectTitle`),u=r.querySelector(`#dwcStatus`),d=r.querySelector(`#dwcWallet`),f=r.querySelector(`#dwcAccount`),p=r.querySelector(`#dwcNetwork`),m=r.querySelector(`#dwcCopy`),h=r.querySelector(`#dwcProviders`),g=r.querySelector(`#dwcPrimary`),_=r.querySelector(`#dwcHint`),r.addEventListener(`click`,async n=>{let i=n.target;if(!i)return;if(i===r){T();return}let a=i.closest(`button[data-action]`);if(!a)return;let o=a.getAttribute(`data-action`)||``;if(o===`close`){T();return}let s=e.state,c=M(s);if(o===`copy`){let e=s.selectedProfile?.account||s.profiles?.[0]?.account||``;if(!e)return;S(await qe(e)?`Copied`:`Copy failed`);return}if(o===`select-provider`){let t=a.getAttribute(`data-provider-id`)||``;if(!t)return;await e.selectProvider(t);return}if(o===`primary`){if(c===`missing`){t.installUrl&&window.open(t.installUrl,`_blank`,`noopener,noreferrer`);return}if(s.availableProviders.length>0&&!s.providerId)return;if(c===`connected`){await e.disconnect();return}await e.connect(t.connectOptions)}}),i=e.subscribe(e=>C(e)),C(e.state))},x=e=>{e.key===`Escape`&&T()},S=e=>{_&&(_.textContent=e,window.setTimeout(()=>{_&&(_.textContent=``)},1200))},C=e=>{let r=M(e),i=e.selectedProfile?.account||e.profiles?.[0]?.account||``,o=He(e),s=e.availableProviders.length>0&&!e.providerId;if(l){let e=(t.appName||``).trim();r===`connected`?l.textContent=e?`Connected to ${e}`:`Wallet`:s?l.textContent=e?`Choose a wallet for ${e}`:`Choose wallet`:l.textContent=Ye(e)}u&&(u.textContent=s?`Choose wallet`:Ge[r]),d&&(d.textContent=Je(e)),f&&(f.textContent=i?Ue(i,10,8):`—`),p&&(p.textContent=o||`—`),m&&(m.hidden=!i),y(e),g&&(g.classList.toggle(`dconnect-btn-destructive`,r===`connected`),g.classList.toggle(`dconnect-btn-primary`,r!==`connected`),g.textContent=s?`Select wallet`:Ke[r],g.disabled=s),_&&(s?_.textContent=`Choose which Dusk wallet this site should use.`:!e.installed&&t.installUrl&&(_.textContent=`Install a compatible Dusk wallet to continue.`)),a&&n&&c&&c!==`connected`&&r===`connected`&&T(),c=r},ee=()=>{b(),!(!r||a)&&(s!==null&&(window.clearTimeout(s),s=null),o=!1,r.removeAttribute(`data-state`),r.isConnected||document.body.appendChild(r),a=!0,c=M(e.state),r.focus(),window.addEventListener(`keydown`,x))},w=()=>{r&&(s!==null&&(window.clearTimeout(s),s=null),o=!1,r.removeAttribute(`data-state`),r.remove())},T=(e=!1)=>{if(!(!r||!a&&!o)){if(a=!1,o=!0,window.removeEventListener(`keydown`,x),e){w();return}r.setAttribute(`data-state`,`closing`),s!==null&&window.clearTimeout(s),s=window.setTimeout(()=>w(),180)}};return{open:ee,close:T,destroy:()=>{T(!0);try{i?.()}catch{}i=null,r=null,l=u=d=f=p=_=null,m=g=null,h=null},isOpen:()=>a}}function N(e){return String(e).replaceAll(`&`,`&amp;`).replaceAll(`<`,`&lt;`).replaceAll(`>`,`&gt;`).replaceAll(`"`,`&quot;`).replaceAll(`'`,`&#39;`)}function tt(e){return e!==null&&e.toLowerCase()!==`false`}var nt=class extends HTMLElement{static get observedAttributes(){return[`app-name`,`install-url`,`close-on-connect`,`hide-network`,`connect-text`,`locked-text`,`install-text`,`theme`,`size`,`variant`]}_shadow;_wallet=null;_modal=null;_walletOptions;_connectOptions;_ownsWallet=!1;_ownsModal=!1;_unsub=null;_btn=null;_avatar=null;_label=null;_net=null;_latest=null;constructor(){super(),this._shadow=this.attachShadow({mode:`open`})}get state(){return this._latest?{...this._latest,accounts:[...this._latest.accounts],profiles:this._latest.profiles.map(e=>({...e})),selectedProfile:this._latest.selectedProfile?{...this._latest.selectedProfile}:null}:null}get wallet(){return this._wallet}set wallet(e){e!==this._wallet&&this._setWallet(e,!1)}get modal(){return this._modal}set modal(e){if(e!==this._modal){if(this._ownsModal&&this._modal)try{this._modal.destroy()}catch{}this._modal=e,this._ownsModal=!1}}get walletOptions(){return this._walletOptions}set walletOptions(e){this._walletOptions=e}get connectOptions(){return this._connectOptions}set connectOptions(e){if(this._connectOptions=e,this._ownsModal&&this._modal){try{this._modal.destroy()}catch{}this._modal=null,this._ownsModal=!1}}open(){this._ensureWalletAndModal(),this._modal?.open()}close(){this._modal?.close()}destroy(){if(this._unsub){try{this._unsub()}catch{}this._unsub=null}if(this._ownsModal&&this._modal)try{this._modal.destroy()}catch{}if(this._modal=null,this._ownsModal=!1,this._ownsWallet&&this._wallet)try{this._wallet.destroy()}catch{}this._wallet=null,this._ownsWallet=!1,this._latest=null}connectedCallback(){this._renderShell(),this._ensureWalletAndModal(),this._update(this._wallet?.state??this._latest)}disconnectedCallback(){this.destroy()}attributeChangedCallback(){if(this._applyAttrsToShell(),this._update(this._latest),this._ownsModal&&this._modal){try{this._modal.destroy()}catch{}this._modal=null,this._ownsModal=!1}}_renderShell(){if(this._btn)return;let e=`
      ${We}

      :host {
        display: inline-block;
      }

      button {
        appearance: none;
        display: inline-flex;
        align-items: center;
        gap: 10px;
        max-width: 100%;
        min-width: 0;
        cursor: pointer;
        user-select: none;
        white-space: nowrap;
        -webkit-tap-highlight-color: transparent;

        border-radius: var(--dconnect-radius);
        padding: 10px 14px;
        font-family: var(--dconnect-font-sans);
        font-weight: 500;
        font-size: 13px;
        line-height: 1;
        letter-spacing: 0;

        border: 1px solid var(--dconnect-button-border);
        background: var(--dconnect-button-bg);
        color: var(--dconnect-foreground);
        box-shadow: 0 0 0 rgba(0, 0, 0, 0);

        transition:
          transform var(--dconnect-dur-fast) var(--dconnect-ease),
          background var(--dconnect-dur-base) var(--dconnect-ease),
          border-color var(--dconnect-dur-base) var(--dconnect-ease),
          box-shadow var(--dconnect-dur-base) var(--dconnect-ease);
      }

      button:hover {
        transform: translateY(-1px);
        background: var(--dconnect-button-hover);
        border-color: var(--dconnect-border-strong);
        box-shadow: var(--dconnect-shadow-hover);
      }

      button:active {
        transform: translateY(1px);
      }

      button:focus-visible {
        outline: none;
        box-shadow: var(--dconnect-shadow-focus);
      }

      button:disabled {
        opacity: 0.55;
        cursor: not-allowed;
        box-shadow: none;
      }

      /* Primary treatment when not connected (solid variant) */
      button[data-variant="solid"][data-status="missing"],
      button[data-variant="solid"][data-status="disconnected"],
      button[data-variant="solid"][data-status="locked"] {
        color: var(--dconnect-primary-foreground);
        border-color: transparent;
        background: var(--dconnect-primary-gradient);
      }

      button[data-variant="solid"][data-status="missing"]:hover,
      button[data-variant="solid"][data-status="disconnected"]:hover,
      button[data-variant="solid"][data-status="locked"]:hover {
        background: var(--dconnect-primary-gradient-hover);
        box-shadow: var(--dconnect-shadow);
      }

      .avatar {
        flex: 0 0 auto;
        width: 18px;
        height: 18px;
        display: inline-flex;
        align-items: center;
        justify-content: center;
        font-family: var(--dconnect-font-mono);
        font-weight: 500;
        font-size: 12px;
        color: var(--dconnect-avatar-fg);
        background: transparent;
        transition:
          color var(--dconnect-dur-base) var(--dconnect-ease),
          transform var(--dconnect-dur-fast) var(--dconnect-ease);
      }

      .avatar::before {
        content: "";
        width: 16px;
        height: 16px;
        display: block;
        background: currentColor;
        -webkit-mask: var(--dconnect-logo-mark) center / contain no-repeat;
        mask: var(--dconnect-logo-mark) center / contain no-repeat;
        transform-origin: center;
      }

      button:hover .avatar {
        color: var(--dconnect-primary-hover);
      }

      button:hover .avatar::before {
        animation: dconnect-logo-pulse 540ms var(--dconnect-ease-out);
      }

      button[data-variant="solid"][data-status="missing"] .avatar,
      button[data-variant="solid"][data-status="disconnected"] .avatar,
      button[data-variant="solid"][data-status="locked"] .avatar {
        color: var(--dconnect-primary-foreground);
      }

      button[data-variant="solid"][data-status="missing"]:hover .avatar,
      button[data-variant="solid"][data-status="disconnected"]:hover .avatar,
      button[data-variant="solid"][data-status="locked"]:hover .avatar {
        color: var(--dconnect-primary-foreground);
      }

      .label {
        display: inline-flex;
        align-items: center;
        min-width: 0;
        max-width: 18ch;
        overflow: hidden;
        text-overflow: ellipsis;
      }

      .net {
        flex: 0 0 auto;
        padding: 4px 7px;
        border-radius: var(--dconnect-radius-sm);
        font-family: var(--dconnect-font-mono);
        font-weight: 500;
        font-size: 10px;
        line-height: 1;
        letter-spacing: 0.06em;
        text-transform: uppercase;
        border: 1px solid rgba(113, 177, 255, 0.28);
        background: rgba(113, 177, 255, 0.12);
        color: var(--dconnect-primary);
        transition:
          background var(--dconnect-dur-base) var(--dconnect-ease),
          border-color var(--dconnect-dur-base) var(--dconnect-ease),
          color var(--dconnect-dur-base) var(--dconnect-ease);
      }

      button:hover .net {
        border-color: rgba(113, 177, 255, 0.42);
        background: rgba(113, 177, 255, 0.18);
      }

      /* size */
      button[data-size="sm"] {
        padding: 8px 12px;
        font-size: 12px;
      }
      button[data-size="sm"] .avatar {
        width: 17px;
        height: 17px;
      }
      button[data-size="lg"] {
        padding: 12px 16px;
        font-size: 14px;
      }
      button[data-size="lg"] .avatar {
        width: 20px;
        height: 20px;
      }

      /* variant */
      button[data-variant="outline"] {
        background: transparent;
        border-color: var(--dconnect-border-strong);
      }
      button[data-variant="solid"] {
        background: var(--dconnect-button-bg);
      }
    `;this._shadow.innerHTML=`
      <style>${e}</style>
      <button type="button">
        <span class="avatar" part="avatar" aria-hidden="true"></span>
        <span class="label" part="label">Connect wallet</span>
        <span class="net" part="network"></span>
      </button>
    `,this._btn=this._shadow.querySelector(`button`),this._avatar=this._shadow.querySelector(`.avatar`),this._label=this._shadow.querySelector(`.label`),this._net=this._shadow.querySelector(`.net`),this._btn?.addEventListener(`click`,()=>this._onClick()),this._applyAttrsToShell()}_applyAttrsToShell(){if(!this._btn)return;let e=(this.getAttribute(`size`)||`md`).toLowerCase(),t=(this.getAttribute(`variant`)||`solid`).toLowerCase();this._btn.setAttribute(`data-size`,e),this._btn.setAttribute(`data-variant`,t)}_onClick(){if(this._ensureWalletAndModal(),M(this._wallet?.state??this._latest)===`missing`){let e=this.getAttribute(`install-url`)||``;if(e){window.open(e,`_blank`,`noopener,noreferrer`);return}}this._modal?.open()}_ensureWalletAndModal(){if(this._wallet||this._setWallet(de(this._walletOptions),!0),!this._modal&&this._wallet){let e=this.getAttribute(`app-name`)||``,t=this.getAttribute(`install-url`)||``,n=(this.getAttribute(`theme`)||`auto`).toLowerCase(),r={};e&&(r.appName=e),t&&(r.installUrl=t),(n===`dark`||n===`light`)&&(r.theme=n),this._connectOptions&&(r.connectOptions=this._connectOptions),this.hasAttribute(`close-on-connect`)&&(r.closeOnConnect=tt(this.getAttribute(`close-on-connect`))),this._modal=et(this._wallet,r),this._ownsModal=!0}}_setWallet(e,t){if(this._unsub){try{this._unsub()}catch{}this._unsub=null}if(this._ownsModal&&this._modal){try{this._modal.destroy()}catch{}this._modal=null,this._ownsModal=!1}if(this._ownsWallet&&this._wallet)try{this._wallet.destroy()}catch{}this._wallet=e,this._ownsWallet=t,this._wallet?(this._unsub=this._wallet.subscribe(e=>this._update(e)),this._wallet.ready().catch(()=>{}),this._update(this._wallet.state)):this._update(null)}_update(e){this._latest=e;try{this.dispatchEvent(new CustomEvent(`dusk-state`,{detail:e,bubbles:!0,composed:!0}))}catch{}let t=M(e);if(this._btn&&this._btn.setAttribute(`data-status`,t),!this._label||!this._avatar||!this._net)return;let n=this.getAttribute(`connect-text`)||`Connect wallet`,r=this.getAttribute(`locked-text`)||`Unlock wallet`,i=this.getAttribute(`install-text`)||`Install wallet`;if(tt(this.getAttribute(`hide-network`))||!e||t!==`connected`)this._net.textContent=``,this._net.style.display=`none`;else{let t=He(e);this._net.textContent=t,this._net.style.display=t?`inline-flex`:`none`}let a=e=>{this._label.textContent=e,this._avatar.textContent=``,this._avatar.style.background=``};if(t!==`connected`){a(t===`missing`?i:t===`locked`?r:n);return}let o=e?.selectedProfile?.account||e?.profiles?.[0]?.account||``;this._label.textContent=o?Ue(o,6,4):`Connected`,this._avatar.textContent=``,this._avatar.style.background=``}};function rt(e=`dusk-connect-button`){typeof window>`u`||customElements.get(e)||customElements.define(e,nt)}try{rt()}catch{}var it=``+new URL(`dario-regular-new-C7_av__A.png`,import.meta.url).href,at=``+new URL(`dario-super-new-aEZibU42.png`,import.meta.url).href,ot=``+new URL(`dario-fire-new-B7iX5xU1.png`,import.meta.url).href,st=``+new URL(`dario-cape-new-l6et7O2w.png`,import.meta.url).href,ct=``+new URL(`dario-gameover-new-D7OxO10m.png`,import.meta.url).href;rt();var lt=`0x8d5abb7e42b7a21a885efd69b0af5e000e82669ac03dd43963bae78bbf30b32f`,ut=`https://testnet.nodes.dusk.network`,dt=`./data_driver.wasm?v=${Date.now()}`,P=/^0x[0-9a-fA-F]{64}$/.test(lt),F=[{name:`Regular`,sprite:it},{name:`Super`,sprite:at},{name:`Fire`,sprite:ot},{name:`Cape`,sprite:st},{name:`Game Over`,sprite:ct}],ft={0:{label:`Espresso`,emoji:`☕`},1:{label:`Chili`,emoji:`🌶️`},2:{label:`Cape`,emoji:`🧣`},3:{label:`Damage`,emoji:`💥`},4:{label:`Revive`,emoji:`💙`}},I=P?Ve({nodeUrl:ut,chain:{chainId:e.testnet},autoConnect:!0,contracts:{dario:{contractId:lt,driverUrl:dt,name:`Dario FSM`,methodSigs:{current_state:`current_state()`,revive_count:`revive_count()`,current_state_for:`current_state_for(String)`,revive_count_for:`revive_count_for(String)`,handle_event:`handle_event(u32)`}}}}):null,L=I?.wallet??null,R=I?.contract(`dario`)??null,z=document.getElementById(`connectBtn`);z&&L&&(z.wallet=L);var B=e=>document.getElementById(e),V=B(`darioSprite`),H=document.querySelector(`.stage`),pt=B(`hudState`),mt=B(`hudRevives`),ht=B(`hudAccount`),U=B(`hint`),gt=B(`deadOverlay`),_t=B(`startOverlay`),vt=B(`startText`),W=B(`startBtn`),yt=B(`pendingOverlay`),G=B(`pendingText`),K=B(`actions`),bt=B(`reviveBig`),q={state:null,revives:null,pending:!1,pendingPhase:``,lastAction:null,ready:!1,error:null},xt=``,J=null,Y=0;function X(){return L?.state.selectedProfile?.account||L?.state.accounts?.[0]||``}function St(e){let t=String(e||``);return t?t.length<=18?t:`${t.slice(0,10)}...${t.slice(-6)}`:`Not connected`}function Z(){return!!(L?.state.authorized&&X())}function Ct(e=q.state){return Number(e)===4}function wt(e){let t=Number(e);return Number.isFinite(t)&&F[t]?F[t]:F[0]}function Tt(e,t){e&&(e.textContent=t)}function Q(){let e=X(),t=Z(),n=Ct();H&&(H.dataset.state=String(q.state??0));let r=wt(q.state);if(V&&V.getAttribute(`src`)!==r.sprite&&(V.style.opacity=`0`,window.setTimeout(()=>{V.setAttribute(`src`,r.sprite),V.style.opacity=`1`},120)),Tt(pt,q.state==null?`-`:r.name),Tt(mt,q.revives==null?`Revives -`:`Revives ${q.revives}`),Tt(ht,St(e)),yt&&(yt.hidden=!q.pending),_t&&(_t.hidden=P&&t||q.pending),gt&&(gt.hidden=!t||q.pending||!n),vt&&(vt.textContent=P?`Connect Wallet to play on Testnet.`:`Set VITE_DARIO_CONTRACT_ID and rebuild.`),W&&(W.disabled=!P||!L,W.textContent=P?`Connect Wallet`:`Contract ID Missing`),G)if(!q.pending)G.textContent=`Waiting for finalization...`;else{let e=ft[q.lastAction]||{label:`Move`,emoji:``};q.pendingPhase===`sign`?G.textContent=`Confirm ${e.emoji} ${e.label} in your wallet...`:q.pendingPhase===`submitted`?G.textContent=`Submitted ${e.emoji} ${e.label}. Waiting for execution...`:G.textContent=`Finalizing ${e.emoji} ${e.label} on-chain...`}let i=K?K.querySelectorAll(`[data-event]`):[];for(let e of i){let r=Number(e.getAttribute(`data-event`));e.hidden=!(n?r===4:r!==4),e.disabled=!P||!q.ready||!t||q.pending}bt&&(bt.disabled=!P||!q.ready||!t||q.pending),U&&(P?q.error?U.textContent=q.error:t?q.ready?q.pending?U.textContent=q.pendingPhase===`sign`?`Confirm the transaction in your wallet...`:`Waiting for on-chain execution...`:n?U.textContent=`Revive Dario to continue.`:U.textContent=`Choose an action.`:U.textContent=`Loading data-driver.`:U.textContent=`Connect your wallet to play.`:U.textContent=`Set VITE_DARIO_CONTRACT_ID and rebuild.`)}async function $(){if(!R||!Z()){q.state=null,q.revives=null,Q();return}if(J)return J;J=(async()=>{try{let e=X(),t=q.state,[n,r]=await Promise.all([R.call.current_state_for(e),R.call.revive_count_for(e)]);q.state=Number(n),q.revives=Number(r),Y=0,q.error=null,t!=null&&q.state!==t&&Et(t,q.state)}catch{Y++,q.state==null||Y>=3?q.error=`Unable to sync on-chain state.`:Y>=2?q.error=`Network hiccup... retrying.`:q.error=null}finally{Q()}})();try{await J}finally{J=null}}function Et(e,t){if(!H)return;let n=``;Number(t)===4?n=`hit`:Number(e)===4&&Number(t)===0?n=`revive`:Number(t)===1?n=`spark`:Number(t)===2?n=`ember`:Number(t)===3&&(n=`wind`),n&&(H.dataset.fx=n,window.setTimeout(()=>{H.dataset.fx===n&&delete H.dataset.fx},750))}async function Dt(){if(L)try{if(typeof z?.open==`function`)z.open();else if(z?.shadowRoot?.querySelector){let e=z.shadowRoot.querySelector(`button`);e?.click?e.click():await L.connect()}else await L.connect()}catch{}}async function Ot(){if(L)try{await L.connect()}catch{}}async function kt(e){if(!(!R||q.pending||!P)&&(q.lastAction=e,!(!Z()&&(await Ot(),!Z())))){q.pending=!0,q.pendingPhase=`sign`,q.error=null,Q();try{let t=await R.write.handle_event(e,{amount:`0`,deposit:`0`}),n=t.onStatus(e=>{e.status===`submitted`&&(q.pendingPhase=`submitted`),e.status===`executing`&&(q.pendingPhase=`executing`),(e.status===`failed`||e.status===`timeout`)&&e.receipt?.error&&(q.error=e.receipt.error),Q()}),r;try{r=await t.wait({timeoutMs:6e4})}finally{n()}!r.ok&&r.error&&(q.error=r.error),await $(),r.status===`timeout`&&(q.error=`Still processing... it may take a bit longer.`,Q())}catch{q.error=`Transaction rejected or failed.`}finally{q.pending=!1,q.pendingPhase=``,Q()}}}K?.addEventListener(`click`,e=>{let t=e.target?.closest?.(`[data-event]`);if(!t)return;let n=Number(t.getAttribute(`data-event`));Number.isFinite(n)&&kt(n)}),bt?.addEventListener(`click`,()=>kt(4)),W?.addEventListener(`click`,Dt);function At(){let e=X();Q(),e&&e!==xt&&(xt=e,$())}async function jt(){if(Q(),!(!I||!L)){L.subscribe(At),await L.ready(),At();try{await I.driver(dt),q.ready=!0}catch{q.error=`Missing or incompatible data_driver.wasm.`}await $(),window.setInterval(()=>{q.pending||$()},8e3)}}jt();
//# sourceMappingURL=index-BpXJ1E_v.js.map