pub mod aptos;
pub mod sui;

/// This is where the common resolver should be defined. 
/// Essentially, it will be composed of a Sui resolver and an Aptos resolver.
/// First, we will check for an Aptos struct.
/// Then, we will check for a Sui object.

// todo: skeleton
pub struct CommonResolver<'state> {
    aptos_resolver : AptosResolver<'state>,
    sui_resolver : SuiResolver<'state>
}

// todo: skeleton
impl <'state>ResourceResolver for CommonResolver<'state> {

    fn get_resource() {

        // aptos resolver takes priority
        match self.aptos_resolver.get_resource() {
            Some(resource) => Some(resource),
            None=>{
                self.sui_resolver.get_resource()
            }
        }

    }

}