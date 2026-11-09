import { useEffect, useState } from 'react';
import { Checkbox, Dropdown, Spin, Typography } from '@douyinfe/semi-ui';
import { IconChevronDown } from '@douyinfe/semi-icons';
import { apiGet } from '../api';
import { useControlledField } from '../framework/useControlledField';

const { Text } = Typography;

interface AuthorityNode {
  authorityId?: number;
  authoritySn?: string;
  authorityName?: string;
  id?: number;
  name?: string;
  children?: AuthorityNode[];
  childAuth?: Array<{ authSn?: string; authName?: string }>;
}

interface AuthorityTreeProps {
  value?: string[];
  onChange: (value: string[]) => void;
}

function nodeSn(node: AuthorityNode) {
  return node.authoritySn || `authority-${node.authorityId || node.id}`;
}

function nodeName(node: AuthorityNode) {
  return node.authorityName || node.name || '';
}

function collectSns(node: AuthorityNode, out: string[] = []) {
  const sn = nodeSn(node);
  if (sn) out.push(sn);
  (node.childAuth || []).forEach((item) => item.authSn && out.push(item.authSn));
  (node.children || []).forEach((child) => collectSns(child, out));
  return out;
}

export default function AuthorityTree({ value = [], onChange }: AuthorityTreeProps) {
  const [tree, setTree] = useState<AuthorityNode[]>([]);
  const [loading, setLoading] = useState(false);
  const [selected, apply] = useControlledField<string[]>(value, onChange);

  useEffect(() => {
    setLoading(true);
    apiGet<AuthorityNode[]>('/authority/authority/getAllAuthority', { adminType: 'admin' })
      .then((result) => setTree(result || []))
      .catch(() => undefined)
      .finally(() => setLoading(false));
  }, []);

  const allSns = collectSns({ children: tree } as AuthorityNode);
  const effective = selected.includes('all') ? allSns : selected;

  const toggleNode = (node: AuthorityNode, checked: boolean) => {
    const sns = collectSns(node);
    const next = new Set(effective);
    sns.forEach((sn) => (checked ? next.add(sn) : next.delete(sn)));
    apply(Array.from(next));
  };

  const toggleSingle = (sn: string, checked: boolean) => {
    const next = new Set(effective);
    if (checked) next.add(sn);
    else next.delete(sn);
    apply(Array.from(next));
  };

  const childrenOf = (node: AuthorityNode) => node.children || [];

  const renderActions = (node: AuthorityNode) => {
    const childAuth = node.childAuth || [];
    if (!childAuth.length) return null;
    return (
      <Dropdown
        trigger="click"
        position="bottomLeft"
        contentClassName="authority-actions-popover"
        content={
          <div className="authority-child-actions">
            {childAuth.map((auth) => (
              <Checkbox
                key={auth.authSn}
                checked={Boolean(auth.authSn && effective.includes(auth.authSn))}
                onChange={(event) => {
                  if (auth.authSn) {
                    toggleSingle(auth.authSn, Boolean(event.target.checked));
                  }
                }}
              >
                {auth.authName}
              </Checkbox>
            ))}
          </div>
        }
      >
        <span className="authority-child-more">
          <IconChevronDown size="small" />
        </span>
      </Dropdown>
    );
  };

  const renderChild = (node: AuthorityNode) => {
    const sn = nodeSn(node);
    const grandchildren = childrenOf(node);
    return (
      <div className="authority-child" key={sn}>
        <div className="authority-child-main">
          <Checkbox
            checked={effective.includes(sn)}
            onChange={(event) => toggleNode(node, Boolean(event.target.checked))}
          >
            {nodeName(node)}
          </Checkbox>
          {renderActions(node)}
        </div>
        {grandchildren.length > 0 && (
          <div className="authority-grandchildren">
            {grandchildren.map((grandchild) => (
              <div className="authority-grandchild" key={nodeSn(grandchild)}>
                <Checkbox
                  checked={effective.includes(nodeSn(grandchild))}
                  onChange={(event) => toggleNode(grandchild, Boolean(event.target.checked))}
                >
                  {nodeName(grandchild)}
                </Checkbox>
                {renderActions(grandchild)}
              </div>
            ))}
          </div>
        )}
      </div>
    );
  };

  return (
    <Spin spinning={loading}>
      <div className="authority-select">
        <div className="authority-select-head">
          <Checkbox
            checked={selected.includes('all')}
            onChange={(event) => apply(event.target.checked ? ['all'] : [])}
          >
            全选
          </Checkbox>
          <Text type="tertiary" size="small">
            权限列表
          </Text>
        </div>
        <div className="authority-select-body">
          {tree.map((parent) => (
            <div className="authority-row" key={nodeSn(parent)}>
              <div className="authority-row-left">
                <Checkbox
                  checked={effective.includes(nodeSn(parent))}
                  onChange={(event) => toggleNode(parent, Boolean(event.target.checked))}
                >
                  {nodeName(parent)}
                </Checkbox>
              </div>
              <div className="authority-row-right">
                {childrenOf(parent).length ? (
                  childrenOf(parent).map((child) => renderChild(child))
                ) : (
                  <Text type="tertiary" size="small">
                    无子权限
                  </Text>
                )}
              </div>
            </div>
          ))}
        </div>
      </div>
    </Spin>
  );
}
