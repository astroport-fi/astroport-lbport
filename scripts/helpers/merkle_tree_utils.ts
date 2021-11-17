import  {Merkle_Tree}  from "./merkle_tree.js";
import airdropdataTerra from "./airdrop_data/airdrop_recepients.json";

const MERKLE_ROOTS = 2;

// TERRA ECOSYSTEM AIRDROP :: RETURNS ROOTS OF THE MERKLE TREES FOR TERRA USERS
export async function getMerkleRoots() { 
    let merkle_roots = [];
    let n = MERKLE_ROOTS;
  
    for (let i=0; i<n; i++ ) {
        let terra_data = prepareDataForMerkleTree(airdropdataTerra.data , i * Math.round(airdropdataTerra.data.length/n) , (i+1) * Math.round(airdropdataTerra.data.length/n)  );
        let airdrop_tree = new Merkle_Tree(terra_data);
        let terra_merkle_root = airdrop_tree.getMerkleRoot();
        merkle_roots.push(terra_merkle_root);            
    }
  
    return merkle_roots;
  }
  



// AIRDROP :: RETURNS MERKLE PROOF
export function get_MerkleProof( leaf: {address: string; amount: string;} ) {
    let merkle_trees = [];
    let n = MERKLE_ROOTS;
  
    for (let i=0; i<n; i++ ) {
        let terra = prepareDataForMerkleTree(airdropdataTerra.data , i * Math.round(airdropdataTerra.data.length/n) , (i+1) * Math.round(airdropdataTerra.data.length/n)  );
        let terra_merkle_tree = new Merkle_Tree(terra);
        merkle_trees.push(terra_merkle_tree);            
    }
  
    let proof = [];
    for (let i=0; i<merkle_trees.length; i++ ) {
        proof = merkle_trees[i].getMerkleProof( leaf );
        if (proof.length > 1) {
          return { "proof":proof, "root_index":i }; 
        }
    }
    return { "proof":null, "root_index":-1 }; 
  }  

// PREPARE DATA FOR THE MERKLE TREE
export function prepareDataForMerkleTree( data:(string | number)[][], str:number, end:number ) { 
    let dataArray = [];
    for ( let i=str; i < end; i++  ) {  
        let dataObj = JSON.parse( JSON.stringify(data[i]) );
        let ac = { "address":dataObj[0], "amount":dataObj[1].toString() };
        dataArray.push(ac);
    }
    return dataArray;
}

